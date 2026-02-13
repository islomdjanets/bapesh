// use crate::date::{now, parse_from_rfc3339, to_rfc3339};

use crate::env;

pub async fn new_action(action_type: &str, user_id: i64, client: &reqwest::Client) -> Result<(), Box<dyn std::error::Error>> {

    let host = env::get("TASKER_HOST").expect("TASKER_HOST IS NOT SETUP");

    let result = client
        .post(format!("{host}/actions/{action_type}/{user_id}"))
        .send()
        .await?;
        // let now = parse_from_rfc3339(
        //     &now().to_string()
        // ).unwrap();
        // // now.setHours(0, 0, 0, 0);
        // let timestamp = to_rfc3339(now);

        // const key = "actions";
        // // const actions = DATA.$(key).get(timestamp) || [];
        // await DATA.update_map(key, timestamp, action_type);
        // // console.log(key, DATA.$(key).get(timestamp));


        // format: {action_type}:{hour-minute}

        Ok(())
}