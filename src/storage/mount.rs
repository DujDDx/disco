//! Mount detection and disk matching

use crate::domain::disk::{Disk, DiskId, MountStatus, MatchResult};
use crate::storage::platform::DiskDetector;
use crate::persistence::disk_repo::DiskRepo;
use crate::Result;

/// Mount checker for disk status
pub struct MountChecker<'a> {
    disk_repo: &'a DiskRepo<'a>,
    detector: &'a dyn DiskDetector,
}

impl<'a> MountChecker<'a> {
    pub fn new(disk_repo: &'a DiskRepo<'a>, detector: &'a dyn DiskDetector) -> Self {
        Self { disk_repo, detector }
    }

    /// Check if a disk is currently mounted
    pub fn is_mounted(&self, disk_id: &DiskId) -> bool {
        self.get_mount_status(disk_id) == MountStatus::Connected
    }

    /// Get mount status of a disk
    pub fn get_mount_status(&self, disk_id: &DiskId) -> MountStatus {
        let disk = self.disk_repo.get_disk_by_id(disk_id);
        match disk {
            Ok(disk) => {
                match self.find_mount_point(&disk) {
                    Ok(Some(_)) => MountStatus::Connected,
                    Ok(None) => MountStatus::Offline,
                    Err(e) => {
                        tracing::warn!("Error finding mount point for disk {}: {}", disk_id, e);
                        MountStatus::Offline
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Error getting disk {}: {}", disk_id, e);
                MountStatus::Offline
            }
        }
    }

    /// Find current mount point for a disk by matching identity
    pub fn find_mount_point(&self, disk: &Disk) -> Result<Option<String>> {
        let mount_points = self.detector.list_mount_points()?;
        let mut best_match: Option<(String, MatchResult)> = None;

        for mount in mount_points {
            let identity = self.detector.detect_identity(&mount)?;

            // Use tolerance matching for better detection
            let match_result = disk.identity.matches_with_tolerance(&identity);

            if match_result.is_match() {
                tracing::debug!(
                    "Disk {} matched mount {} with confidence {}: {}",
                    disk.disk_id, mount, match_result.confidence(), match_result.reason()
                );

                // Keep the best match (highest confidence)
                if best_match.is_none() || match_result.confidence() > best_match.as_ref().unwrap().1.confidence() {
                    best_match = Some((mount, match_result));
                }
            }
        }

        if let Some((mount, result)) = best_match {
            tracing::info!("Best match for disk {}: {} at {}", disk.disk_id, result.reason(), mount);
            Ok(Some(mount))
        } else {
            tracing::debug!("No mount point found for disk {}", disk.disk_id);
            Ok(None)
        }
    }

    /// Refresh mount status for all disks with detailed diagnostics
    pub fn refresh_all(&self) -> Result<Vec<(DiskId, MountStatus, Option<String>, Option<String>)>> {
        let disks = self.disk_repo.list_disks()?;
        let mount_points = self.detector.list_mount_points()?;

        let mut results = Vec::new();

        // Build a map of mount points to identities
        let mount_identities: Vec<(String, crate::domain::disk::DiskIdentity)> = mount_points
            .iter()
            .filter_map(|m| {
                self.detector.detect_identity(m).ok().map(|id| (m.clone(), id))
            })
            .collect();

        tracing::info!("Refreshing mount status for {} disks against {} mount points", disks.len(), mount_identities.len());

        for disk in disks {
            let mut status = MountStatus::Offline;
            let mut mount_point: Option<String> = None;
            let mut diagnostic: Option<String> = None;

            // Find best match
            let mut best_match: Option<(String, MatchResult)> = None;

            for (mount, identity) in &mount_identities {
                let match_result = disk.identity.matches_with_tolerance(identity);

                if match_result.is_match() {
                    if best_match.is_none() || match_result.confidence() > best_match.as_ref().unwrap().1.confidence() {
                        best_match = Some((mount.clone(), match_result));
                    }
                }
            }

            if let Some((mount, result)) = best_match {
                mount_point = Some(mount.clone());
                status = MountStatus::Connected;
                tracing::info!("Disk {} ({}) connected at {}: {}", disk.disk_id, disk.name, mount, result.reason());
            } else {
                // Check for identity conflict (similar label but different identity)
                for (_mount, identity) in &mount_identities {
                    if disk.identity.volume_label.is_some()
                        && identity.volume_label == disk.identity.volume_label
                        && !disk.identity.matches(identity) {
                        status = MountStatus::IdentityConflict;
                        diagnostic = Some(format!(
                            "Volume '{}' mounted but identity differs - possible reformat or different disk",
                            disk.identity.volume_label.as_deref().unwrap_or("")
                        ));
                        tracing::warn!("Identity conflict detected for disk {}", disk.disk_id);
                        break;
                    }
                }

                // Generate diagnostic for offline disk
                if status == MountStatus::Offline {
                    // Find any mount point that might be related
                    let mut candidates = Vec::new();
                    for (mount, identity) in &mount_identities {
                        if let Some(label) = &disk.identity.volume_label {
                            if identity.volume_label.is_some() {
                                let l1 = label.to_lowercase();
                                let l2 = identity.volume_label.as_ref().unwrap().to_lowercase();
                                if l1.contains(&l2) || l2.contains(&l1) {
                                    candidates.push((mount.clone(), identity.clone()));
                                }
                            }
                        }
                    }

                    if !candidates.is_empty() {
                        diagnostic = Some(format!(
                            "Possible match at: {} (labels similar but identity differs)",
                            candidates.iter().map(|(m, _)| m.clone()).collect::<Vec<_>>().join(", ")
                        ));
                    } else {
                        diagnostic = Some(format!(
                            "No matching mount found. Registered as: '{}' ({})",
                            disk.identity.volume_label.as_deref().unwrap_or("unknown"),
                            format_capacity(disk.identity.capacity_bytes)
                        ));
                    }
                }
            }

            // Update disk in repo
            if let Some(mp) = &mount_point {
                self.disk_repo.update_last_mount_point(&disk.disk_id, mp.clone())?;
            }

            results.push((disk.disk_id.clone(), status, mount_point, diagnostic));
        }

        Ok(results)
    }

    /// Get the disk that owns a mount point
    pub fn get_disk_for_mount(&self, mount_point: &str) -> Result<Option<Disk>> {
        let identity = self.detector.detect_identity(mount_point)?;
        let disks = self.disk_repo.list_disks()?;

        for disk in disks {
            if disk.identity.matches(&identity) {
                return Ok(Some(disk));
            }
        }

        Ok(None)
    }

    /// Force refresh all disks and return detailed diagnostics
    pub fn force_refresh(&self) -> Result<RefreshReport> {
        let disks = self.disk_repo.list_disks()?;
        let mount_points = self.detector.list_mount_points()?;

        tracing::info!("Force refresh: checking {} mount points", mount_points.len());

        // Get all identities with details
        let mount_details: Vec<MountDetail> = mount_points
            .iter()
            .filter_map(|m| {
                self.detector.detect_identity(m).ok().map(|id| {
                    MountDetail {
                        mount_point: m.clone(),
                        identity: id,
                    }
                })
            })
            .collect();

        let mut disk_reports: Vec<DiskRefreshReport> = Vec::new();

        for disk in disks {
            let mut matches: Vec<MatchDetail> = Vec::new();

            for detail in &mount_details {
                let result = disk.identity.matches_with_tolerance(&detail.identity);
                if result.is_match() || disk.identity.volume_label.is_some() && detail.identity.volume_label == disk.identity.volume_label {
                    matches.push(MatchDetail {
                        mount_point: detail.mount_point.clone(),
                        match_result: result,
                        identity: detail.identity.clone(),
                    });
                }
            }

            let status = if matches.iter().any(|m| m.match_result.is_match()) {
                MountStatus::Connected
            } else if matches.iter().any(|m| !m.match_result.is_match() && m.identity.volume_label == disk.identity.volume_label) {
                MountStatus::IdentityConflict
            } else {
                MountStatus::Offline
            };

            let best_mount = matches.iter()
                .filter(|m| m.match_result.is_match())
                .max_by(|a, b| a.match_result.confidence().partial_cmp(&b.match_result.confidence()).unwrap())
                .map(|m| m.mount_point.clone());

            disk_reports.push(DiskRefreshReport {
                disk_id: disk.disk_id.clone(),
                name: disk.name.clone(),
                status,
                mount_point: best_mount.clone(),
                registered_identity: disk.identity.clone(),
                potential_matches: matches,
            });

            // Update last mount point
            if let Some(mp) = best_mount {
                self.disk_repo.update_last_mount_point(&disk.disk_id, mp)?;
            }
        }

        Ok(RefreshReport {
            mount_points: mount_details,
            disk_reports,
        })
    }
}

/// Format capacity for display
fn format_capacity(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Detailed information about a mount point
#[derive(Debug, Clone)]
pub struct MountDetail {
    pub mount_point: String,
    pub identity: crate::domain::disk::DiskIdentity,
}

/// Details about a potential match
#[derive(Debug, Clone)]
pub struct MatchDetail {
    pub mount_point: String,
    pub match_result: MatchResult,
    pub identity: crate::domain::disk::DiskIdentity,
}

/// Report for a single disk after refresh
#[derive(Debug, Clone)]
pub struct DiskRefreshReport {
    pub disk_id: DiskId,
    pub name: String,
    pub status: MountStatus,
    pub mount_point: Option<String>,
    pub registered_identity: crate::domain::disk::DiskIdentity,
    pub potential_matches: Vec<MatchDetail>,
}

/// Full refresh report with all details
#[derive(Debug, Clone)]
pub struct RefreshReport {
    pub mount_points: Vec<MountDetail>,
    pub disk_reports: Vec<DiskRefreshReport>,
}