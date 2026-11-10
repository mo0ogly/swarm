//! Persisted native reviewer transport ceiling; no provider or acceptance.
use swarm_engine::{EngineSettingsValues, ReviewOutputStream, ReviewQuoteOriginal, ReviewReplyState, SettingCap, SettingCount};
const REPLY: &str = r#"{"reason":"Complete fixture reason","criteria":[{"index":1,"verdict":"pass","evidence":"Complete original evidence"}]}"#;
/// Supply a complete native JSONL terminal response; fixture usage is no invoice.
fn response() -> Vec<u8> {
    format!("{}\n{{\"type\":\"turn.completed\"}}\n", serde_json::json!({"type":"item.completed","item":{"type":"agent_message","id":"review","text":REPLY}})).into_bytes()
}
/// Assert actual saved-limit consumer success/failure without processes or mutation.
fn consumed(values: &EngineSettingsValues, accepted: bool) {
    let sources = [ReviewQuoteOriginal { name:"report.md", kind:"source", content:"Complete original evidence" }];
    let mut stream = ReviewOutputStream::from_saved_settings(values, 1, &sources).unwrap();
    let feed = stream.feed(&response());
    if accepted {
        feed.unwrap();
        assert_eq!(stream.finish().unwrap().review.state, ReviewReplyState::Passed);
    } else {
        assert!(feed.is_err());
        assert!(stream.finish().is_err(), "a capped failure must stay failed");
    }
}
/// Saved reviewer policy is independent of planner policy, with inclusive bounds.
#[test]
fn saved_review_response_limit_reaches_stream_without_planner_fallback() {
    let mut values = EngineSettingsValues::installation_defaults().unwrap();
    assert_eq!(values.schema_version(),31);
    assert_eq!(values.review_response_limit().unwrap(),None);
    values.planning_response_max_bytes=Some(SettingCap(Some(SettingCount(0))));
    consumed(&values,true);
    values.review_response_max_bytes=Some(SettingCap(Some(SettingCount(REPLY.len() as i64))));
    consumed(&values,true);
    values.review_response_max_bytes=Some(SettingCap(Some(SettingCount(REPLY.len() as i64-1))));
    consumed(&values,false);
    values.review_response_max_bytes=Some(SettingCap(None));consumed(&values,true);
    values.review_response_max_bytes=Some(SettingCap(Some(SettingCount(0))));consumed(&values,false);
}
/// Historical absence never becomes a default; negative/partial policies refuse.
#[test]
fn legacy_review_response_policy_requires_explicit_complete_adoption() {
    let mut values=EngineSettingsValues::installation_defaults().unwrap();
    // v30/v31 depend on v29: a v28 document carries none of these fields.
    values.agent_cache_miss_ratio_percent=None;values.agent_cache_min_input_tokens=None;values.agent_telemetry_providers=None;values.review_response_max_bytes=None;values.validate().unwrap();assert_eq!(values.schema_version(),28);
    assert_eq!(values.review_response_limit().unwrap_err().reason(),"settings-review-response-not-configured");
    let encoded=swarm_engine::encode_model(&values).unwrap();
    assert_eq!(EngineSettingsValues::decode(&encoded).unwrap(),values);
    let sources=[];
    assert!(ReviewOutputStream::from_saved_settings(&values,0,&sources).is_err());
    values.review_response_max_bytes=Some(SettingCap(Some(SettingCount(-1))));assert!(values.validate().is_err());
    values.review_response_max_bytes=Some(SettingCap(None));values.planning_quota_reason_min_bytes=None;values.planning_quota_reason_max_bytes=None;
    assert_eq!(values.validate().unwrap_err().reason(),"settings-review-response-requires-quota-policy");
}
