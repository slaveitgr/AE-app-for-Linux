use std::time::Duration;

use crate::core::{
    auth::storage::LockedAuthStorage,
    cache::DynamicCache,
    locale::Locale,
    service_layer::{
        ServiceAvailableBuild, ServiceAvailableBuilds, ServiceAvailableBuildsBuilder,
        ServiceAvailableBuildsRequestBuilder, ServiceDownloadUrlMetadata,
        ServiceDownloadUrlRequestBuilder, ServiceGetLegacyCatalogDefsRequestBuilder,
        ServiceLayerClient, ServiceLayerError, ServiceLegacyOffer, SERVICE_REQUEST_AVAILABLEBUILDS,
        SERVICE_REQUEST_DOWNLOADURL, SERVICE_REQUEST_GETLEGACYCATALOGDEFS,
    },
};

pub mod downloader;
pub mod install;
pub mod manager;
pub mod zip;
pub mod zlib;

pub struct ContentService {
    service_layer: ServiceLayerClient,
    request_cache: DynamicCache<String>,
}

// PCWIN is present in the supplied Windows client and the existing commerce
// platform model. PC alone is not sufficient proof of a Windows payload.
fn windows_platform(software: Option<&str>, base: Option<&str>) -> Result<bool, ServiceLayerError> {
    let mut windows = false;
    for value in [software, base]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        match value.to_ascii_uppercase().as_str() {
            "PCWIN" => windows = true,
            "PC" => {}
            _ => {
                return Err(ServiceLayerError::GamePlatform(format!(
                    "unsupported or conflicting platform {value}"
                )))
            }
        }
    }
    Ok(windows)
}

fn select_windows_build<'a>(
    software: Option<&str>,
    base: Option<&str>,
    builds: &'a ServiceAvailableBuilds,
    requested: Option<&str>,
) -> Result<&'a ServiceAvailableBuild, ServiceLayerError> {
    if !windows_platform(software, base)? {
        return Err(ServiceLayerError::GamePlatform(
            "offer has no explicit PCWIN metadata; Windows payload cannot be verified".into(),
        ));
    }
    let live: Vec<_> = builds
        .builds
        .iter()
        .filter(|build| {
            build.download_type().as_ref()
                == Some(&crate::core::service_layer::ServiceDownloadType::Live)
        })
        .collect();
    let check = |build: &ServiceAvailableBuild| {
        windows_platform(
            build.software_platform().as_deref(),
            build.base_platform().as_deref(),
        )
    };
    if let Some(id) = requested {
        let build = builds
            .builds
            .iter()
            .find(|build| build.build_id() == id)
            .ok_or_else(|| {
                ServiceLayerError::GamePlatform(
                    "selected build is not in the current offer metadata".into(),
                )
            })?;
        if check(build)? || (live.len() == 1 && live[0].build_id() == id) {
            return Ok(build);
        }
        return Err(ServiceLayerError::GamePlatform(
            "selected build has no Windows metadata and the live builds are ambiguous".into(),
        ));
    }
    if let Some(build) = live.iter().find(|build| matches!(check(build), Ok(true))) {
        return Ok(build);
    }
    if live.len() == 1 {
        check(live[0])?;
        return Ok(live[0]);
    }
    Err(ServiceLayerError::GamePlatform(
        "no unambiguous Windows LIVE build is available".into(),
    ))
}

impl ContentService {
    pub fn new(auth: LockedAuthStorage) -> Self {
        let request_cache = DynamicCache::new(
            100,
            Duration::from_secs(30 * 60),
            Duration::from_secs(5 * 60),
        );

        Self {
            service_layer: ServiceLayerClient::new(auth),
            request_cache,
        }
    }

    pub async fn available_builds(
        &self,
        offer_id: &str,
    ) -> Result<ServiceAvailableBuilds, ServiceLayerError> {
        let cache_key = "builds_".to_owned() + offer_id;
        if let Some(cached) = self.request_cache.get(&cache_key) {
            return Ok(cached);
        }

        let builds: Vec<ServiceAvailableBuild> = self
            .service_layer
            .request(
                SERVICE_REQUEST_AVAILABLEBUILDS,
                ServiceAvailableBuildsRequestBuilder::default()
                    .offer_id(offer_id.to_owned())
                    .build()?,
            )
            .await?;

        let builds = ServiceAvailableBuildsBuilder::default()
            .builds(builds)
            .build()?;
        self.request_cache.insert(cache_key, builds.clone());
        Ok(builds)
    }

    async fn windows_build(
        &self,
        offer_id: &str,
        requested: Option<&str>,
    ) -> Result<ServiceAvailableBuild, ServiceLayerError> {
        let offers: Vec<ServiceLegacyOffer> = self
            .service_layer
            .request(
                SERVICE_REQUEST_GETLEGACYCATALOGDEFS,
                ServiceGetLegacyCatalogDefsRequestBuilder::default()
                    .offer_ids(vec![offer_id.to_owned()])
                    .locale(Locale::EnUs)
                    .build()?,
            )
            .await?;
        let offer = offers
            .iter()
            .find(|offer| offer.offer_id() == offer_id)
            .ok_or_else(|| {
                ServiceLayerError::GamePlatform("offer platform metadata is unavailable".into())
            })?;
        let builds = self.available_builds(offer_id).await?;
        Ok(select_windows_build(
            offer.software_platform().as_deref(),
            offer.base_platform().as_deref(),
            &builds,
            requested,
        )?
        .clone())
    }

    /// Select a verified Windows build before adding it to a download queue.
    pub async fn windows_live_build(
        &self,
        offer_id: &str,
    ) -> Result<ServiceAvailableBuild, ServiceLayerError> {
        self.windows_build(offer_id, None).await
    }

    pub async fn download_url(
        &self,
        offer_id: &str,
        build_id: Option<&str>,
    ) -> Result<ServiceDownloadUrlMetadata, ServiceLayerError> {
        // Recheck persisted/manual queue entries before using even a cached URL.
        let build = self
            .windows_build(offer_id, build_id.filter(|id| !id.is_empty()))
            .await?;
        let build_id = build.build_id();
        let cache_key = "download_url_".to_owned() + offer_id + "_" + build_id;
        if let Some(cached) = self.request_cache.get(&cache_key) {
            return Ok(cached);
        }

        let url: ServiceDownloadUrlMetadata = self
            .service_layer
            .request(
                SERVICE_REQUEST_DOWNLOADURL,
                ServiceDownloadUrlRequestBuilder::default()
                    .offer_id(offer_id.to_owned())
                    .build_id(build_id.to_owned())
                    .build()?,
            )
            .await?;

        self.request_cache.insert(cache_key, url.clone());
        Ok(url)
    }
}

#[cfg(test)]
mod platform_tests {
    use super::*;

    fn builds(values: serde_json::Value) -> ServiceAvailableBuilds {
        serde_json::from_value(serde_json::json!({"builds": values})).unwrap()
    }

    #[test]
    fn windows_offer_accepts_one_live_build() {
        let values = builds(serde_json::json!([
            {"buildId":"windows-live","downloadType":"LIVE"}
        ]));
        assert_eq!(
            select_windows_build(Some("PCWIN"), Some("PC"), &values, None)
                .unwrap()
                .build_id(),
            "windows-live"
        );
        assert!(
            select_windows_build(Some("PCWIN"), Some("PC"), &values, Some("windows-live")).is_ok()
        );
    }

    #[test]
    fn mac_and_conflicting_platforms_are_rejected() {
        let values = builds(serde_json::json!([
            {"buildId":"mac-live","downloadType":"LIVE","softwarePlatform":"MAC"}
        ]));
        for platform in ["MAC", "MAC_PC", "DARWIN", "OTHER"] {
            assert!(select_windows_build(Some(platform), None, &values, None).is_err());
        }
        assert!(select_windows_build(Some("PCWIN"), Some("MAC"), &values, None).is_err());
        assert!(
            select_windows_build(Some("PCWIN"), Some("PC"), &values, Some("mac-live")).is_err()
        );
    }

    #[test]
    fn missing_offer_and_ambiguous_build_metadata_are_rejected() {
        let values = builds(serde_json::json!([
            {"buildId":"one","downloadType":"LIVE"},
            {"buildId":"two","downloadType":"LIVE"}
        ]));
        assert!(select_windows_build(None, Some("PC"), &values, None).is_err());
        assert!(select_windows_build(Some("PCWIN"), None, &values, None).is_err());
        assert!(select_windows_build(Some("PCWIN"), None, &values, Some("one")).is_err());
    }

    #[test]
    fn explicit_windows_live_build_wins_over_mac() {
        let values = builds(serde_json::json!([
            {"buildId":"mac","downloadType":"LIVE","softwarePlatform":"MAC"},
            {"buildId":"windows","downloadType":"LIVE","softwarePlatform":"PCWIN"}
        ]));
        assert_eq!(
            select_windows_build(Some("PCWIN"), Some("PC"), &values, None)
                .unwrap()
                .build_id(),
            "windows"
        );
    }
}
