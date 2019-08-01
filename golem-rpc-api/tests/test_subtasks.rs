use golem_rpc_api::comp::SubtaskInfo;
use serde_json;

#[test]
fn test_parse_subtasks_result() {
    let json = include_str!("test-subtasks-list.json");

    let subtasks: Vec<SubtaskInfo> = serde_json::from_str(json).unwrap();

    eprintln!("{:?}", subtasks);
}
