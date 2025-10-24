//! Port mapping managers for automatic renewal and lifecycle management
//!
//! This module provides high-level managers for maintaining port mappings:
//! - `PortMappingManager` - PCP/NAT-PMP mapping with automatic renewal
//! - `UpnpMappingManager` - UPnP mapping with automatic cleanup

use super::pcp::try_pcp_mapping_with_protocol;
use super::types::{IpProtocol, MappingError, PortMappingResult};
use super::upnp::{delete_upnp_mapping, try_upnp_mapping_with_protocol};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{debug, error, info};

/// Automatic port mapping manager with renewal
///
/// This manager creates a port mapping and automatically renews it
/// before it expires (at 80% of lifetime).
pub struct PortMappingManager {
    local_port: u16,
    lifetime_secs: u32,
    protocol: IpProtocol,
    current_mapping: Arc<Mutex<Option<PortMappingResult>>>,
    renewal_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl PortMappingManager {
    /// Create a new port mapping manager
    pub fn new(local_port: u16, lifetime_secs: u32, protocol: IpProtocol) -> Self {
        Self {
            local_port,
            lifetime_secs,
            protocol,
            current_mapping: Arc::new(Mutex::new(None)),
            renewal_task: Arc::new(Mutex::new(None)),
        }
    }

    /// Start the port mapping and automatic renewal
    ///
    /// This will create an initial mapping and spawn a background task
    /// to renew it at 80% of the lifetime.
    pub async fn start(&self) -> Result<PortMappingResult, MappingError> {
        info!(
            "Starting port mapping manager for port {} (lifetime: {}s)",
            self.local_port, self.lifetime_secs
        );

        // Create initial mapping
        let mapping = try_pcp_mapping_with_protocol(
            self.local_port,
            self.lifetime_secs,
            self.protocol,
        )
        .await?;

        // Store mapping
        *self.current_mapping.lock().await = Some(mapping.clone());

        // Start renewal task
        self.start_renewal_task().await;

        Ok(mapping)
    }

    /// Stop the port mapping and cancel renewal
    pub async fn stop(&self) -> Result<(), MappingError> {
        info!("Stopping port mapping manager for port {}", self.local_port);

        // Cancel renewal task
        if let Some(task) = self.renewal_task.lock().await.take() {
            task.abort();
        }

        // Delete mapping (lifetime = 0)
        try_pcp_mapping_with_protocol(self.local_port, 0, self.protocol).await?;

        // Clear current mapping
        *self.current_mapping.lock().await = None;

        Ok(())
    }

    /// Get the current mapping (if any)
    pub async fn current_mapping(&self) -> Option<PortMappingResult> {
        self.current_mapping.lock().await.clone()
    }

    /// Start the background renewal task
    async fn start_renewal_task(&self) {
        let local_port = self.local_port;
        let lifetime_secs = self.lifetime_secs;
        let protocol = self.protocol;
        let current_mapping = self.current_mapping.clone();

        // Cancel existing task if any
        if let Some(task) = self.renewal_task.lock().await.take() {
            task.abort();
        }

        // Spawn new renewal task
        let task = tokio::spawn(async move {
            loop {
                // Calculate renewal time (80% of lifetime)
                let renewal_delay_secs = (lifetime_secs as f64 * 0.8) as u64;
                debug!(
                    "Next renewal in {} seconds (80% of {}s lifetime)",
                    renewal_delay_secs, lifetime_secs
                );

                tokio::time::sleep(Duration::from_secs(renewal_delay_secs)).await;

                // Attempt renewal
                info!("Renewing port mapping for port {}", local_port);
                match try_pcp_mapping_with_protocol(local_port, lifetime_secs, protocol).await {
                    Ok(new_mapping) => {
                        info!(
                            "Port mapping renewed: {}:{} (lifetime: {}s)",
                            new_mapping.external_ip,
                            new_mapping.external_port,
                            new_mapping.lifetime_secs
                        );
                        *current_mapping.lock().await = Some(new_mapping);
                    }
                    Err(e) => {
                        error!("Failed to renew port mapping: {}", e);
                        // Continue trying - next renewal will happen after normal interval
                    }
                }
            }
        });

        *self.renewal_task.lock().await = Some(task);
    }
}

impl Drop for PortMappingManager {
    fn drop(&mut self) {
        // Cancel renewal task on drop
        if let Some(task) = self.renewal_task.try_lock().ok().and_then(|mut guard| guard.take()) {
            task.abort();
        }
    }
}

/// UPnP Port Mapping Manager with automatic cleanup
///
/// This manager creates a UPnP mapping and automatically deletes it when dropped.
pub struct UpnpMappingManager {
    local_port: u16,
    protocol: IpProtocol,
    current_mapping: Arc<Mutex<Option<PortMappingResult>>>,
}

impl UpnpMappingManager {
    /// Create a new UPnP mapping manager
    pub fn new(local_port: u16, protocol: IpProtocol) -> Self {
        Self {
            local_port,
            protocol,
            current_mapping: Arc::new(Mutex::new(None)),
        }
    }

    /// Start the port mapping
    pub async fn start(&self, lifetime_secs: u32) -> Result<PortMappingResult, MappingError> {
        info!(
            "Starting UPnP mapping manager for port {} (lifetime: {}s)",
            self.local_port, lifetime_secs
        );

        // Create mapping
        let mapping = try_upnp_mapping_with_protocol(
            self.local_port,
            lifetime_secs,
            self.protocol,
        )
        .await?;

        // Store mapping
        *self.current_mapping.lock().await = Some(mapping.clone());

        Ok(mapping)
    }

    /// Stop the port mapping and delete it
    pub async fn stop(&self) -> Result<(), MappingError> {
        info!("Stopping UPnP mapping manager for port {}", self.local_port);

        // Delete mapping
        delete_upnp_mapping(self.local_port, self.protocol).await?;

        // Clear current mapping
        *self.current_mapping.lock().await = None;

        Ok(())
    }

    /// Get the current mapping (if any)
    pub async fn current_mapping(&self) -> Option<PortMappingResult> {
        self.current_mapping.lock().await.clone()
    }
}

impl Drop for UpnpMappingManager {
    fn drop(&mut self) {
        // Attempt cleanup on drop (best effort)
        let local_port = self.local_port;
        let protocol = self.protocol;

        // Spawn blocking cleanup task
        std::thread::spawn(move || {
            // Search for gateway
            if let Ok(gateway) = igd_next::search_gateway(igd_next::SearchOptions {
                timeout: Some(Duration::from_secs(2)),
                ..Default::default()
            }) {
                let upnp_protocol = match protocol {
                    IpProtocol::TCP => igd_next::PortMappingProtocol::TCP,
                    IpProtocol::UDP => igd_next::PortMappingProtocol::UDP,
                };

                let _ = gateway.remove_port(upnp_protocol, local_port);
                debug!("UPnP mapping cleaned up on drop");
            }
        });
    }
}
