//! Fixed, read-only capability probe from the normal app identity. No caller-selected URI,
//! application launch, subscription or setting write can be requested through this module.
pub fn survey_operations() -> Vec<(&'static str, &'static str, serde_json::Value)> {
    use serde_json::json;
    vec![
        (
            "legacy_audio_volume",
            "luna://com.webos.audio/getVolume",
            json!({}),
        ),
        (
            "legacy_audio_output",
            "luna://com.webos.audio/getSoundOut",
            json!({}),
        ),
        (
            "audio_volume",
            "luna://com.webos.service.audio/getVolume",
            json!({}),
        ),
        (
            "audio_output",
            "luna://com.webos.service.audio/getSoundOutput",
            json!({}),
        ),
        (
            "network",
            "luna://com.webos.service.connectionmanager/getStatus",
            json!({"subscribe":false}),
        ),
        (
            "network_legacy",
            "luna://com.palm.connectionmanager/getStatus",
            json!({"subscribe":false}),
        ),
        (
            "sound_output_setting",
            "luna://com.webos.service.settings/getSystemSettings",
            json!({"category":"sound","keys":["soundOutput"]}),
        ),
        (
            "sound_mode_setting",
            "luna://com.webos.service.settings/getSystemSettings",
            json!({"category":"sound","keys":["soundMode"]}),
        ),
        (
            "sound_output_desc",
            "luna://com.webos.service.settings/getSystemSettingDesc",
            json!({"category":"sound","keys":["soundOutput"]}),
        ),
        (
            "sound_mode_desc",
            "luna://com.webos.service.settings/getSystemSettingDesc",
            json!({"category":"sound","keys":["soundMode"]}),
        ),
        (
            "picture_desc",
            "luna://com.webos.service.settings/getSystemSettingDesc",
            json!({"category":"picture","keys":["pictureMode","backlight","contrast","brightness"]}),
        ),
    ]
}
#[cfg(target_arch = "arm")]
pub fn collect() -> serde_json::Value {
    use plx_platform::webos::ls2;
    use serde_json::json;
    let bus = match ls2::register() {
        Ok(bus) => bus,
        Err(error) => {
            return json!({"registered":false,"error":error.to_string(),"uid":unsafe{libc::geteuid()},"writes":false})
        }
    };
    let mut value = read(&bus);
    let mut survey = serde_json::Map::new();
    for (name, uri, payload) in survey_operations() {
        let result = match bus.call(
            uri,
            &payload.to_string(),
            std::time::Duration::from_millis(700),
        ) {
            Ok(raw) if raw.len() <= 2 * 1024 * 1024 => {
                serde_json::from_str(&raw).unwrap_or_else(|_| json!({"parse_error":true}))
            }
            Ok(_) => json!({"oversize":true}),
            Err(_) => json!({"unavailable":true}),
        };
        survey.insert(name.to_owned(), result);
    }
    value["survey"] = json!(survey);
    value["registered"] = serde_json::json!(true);
    value["uid"] = serde_json::json!(unsafe { libc::geteuid() });
    value
}
#[cfg(target_arch = "arm")]
pub(crate) fn read(bus: &plx_platform::webos::ls2::Registration) -> serde_json::Value {
    use plx_platform::webos::ls2;
    use serde_json::json;
    let operations = [
        (
            "system_info",
            "luna://com.webos.service.tv.systemproperty/getSystemInfo",
            json!({"keys":["modelName","firmwareVersion","sdkVersion"]}),
        ),
        (
            "apps",
            "luna://com.webos.applicationManager/listApps",
            json!({}),
        ),
        (
            "launch_points",
            "luna://com.webos.applicationManager/listLaunchPoints",
            json!({}),
        ),
        (
            "inputs",
            "luna://com.webos.service.eim/getAllInputStatus",
            json!({}),
        ),
        (
            "current_input",
            "luna://com.webos.service.eim/getCurrentInput",
            json!({}),
        ),
        (
            "picture",
            "luna://com.webos.service.settings/getSystemSettings",
            json!({"category":"picture","keys":["pictureMode","backlight","contrast","brightness"]}),
        ),
        (
            "audio_output_before",
            "luna://com.webos.service.audio/getSoundOutput",
            json!({}),
        ),
        ("audio", "luna://com.webos.audio/getVolume", json!({})),
        (
            "audio_output",
            "luna://com.webos.service.audio/getSoundOutput",
            json!({}),
        ),
        (
            "sound_settings",
            "luna://com.webos.service.settings/getSystemSettings",
            json!({"category":"sound","keys":["soundMode","soundOutput"]}),
        ),
        (
            "network",
            "luna://com.webos.service.connectionmanager/getStatus",
            json!({"subscribe":false}),
        ),
    ];
    let mut replies = serde_json::Map::new();
    for (name, uri, payload) in operations.into_iter().map(|(name,uri,payload)|(name.to_owned(),uri,payload))
        .chain(crate::control_reads::operations())
        .chain(crate::sound_reads::operations().into_iter().map(|(name, uri, payload)| (name.to_owned(), uri, payload)))
        .chain(std::iter::once(("picture_after".into(),"luna://com.webos.service.settings/getSystemSettings",json!({"category":"picture","keys":["pictureMode","backlight","contrast","brightness"]})))) {
        let response = bus.call(
            uri,
            &payload.to_string(),
            std::time::Duration::from_millis(700),
        );
        let result = match response {
            Ok(raw) if raw.len() <= 2 * 1024 * 1024 => serde_json::from_str(&raw).unwrap_or_else(|_| json!({"parse_error":true})),
            Ok(_) => json!({"oversize":true}),
            Err(ls2::Fail::Timeout) => json!({"timeout":true}),
            Err(ls2::Fail::Setup {
                stage,
                detail,
                code,
            }) => json!({"setup_error":stage,"detail":detail,"code":code}),
        };
        replies.insert(name.to_owned(), result);
    }
    json!({"registered":true,"uid":unsafe{libc::geteuid()},"writes":false,"replies":replies})
}
#[cfg(not(target_arch = "arm"))]
pub fn collect() -> serde_json::Value {
    serde_json::json!({"registered":false,"unsupported_host":true,"writes":false})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn survey_is_a_fixed_non_subscribing_read_only_catalogue() {
        let operations = survey_operations();
        assert_eq!(operations.len(), 11);
        for (_, uri, payload) in operations {
            assert!(matches!(
                uri.rsplit('/').next(),
                Some(
                    "getVolume"
                        | "getSoundOut"
                        | "getSoundOutput"
                        | "getStatus"
                        | "getSystemSettings"
                        | "getSystemSettingDesc"
                )
            ));
            assert_ne!(
                payload
                    .get("subscribe")
                    .and_then(serde_json::Value::as_bool),
                Some(true)
            );
            assert!(payload.get("appId").is_none() && payload.get("app_id").is_none());
        }
    }
}
