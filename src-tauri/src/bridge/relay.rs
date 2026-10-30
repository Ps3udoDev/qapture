use serde_json::{Value, json};

pub enum Forward {
    Server(String),
}
#[derive(Default)]
pub struct Relay {
    initialize: Option<Value>,
    pending: Vec<Value>,
    replaying: bool,
}

impl Relay {
    pub fn on_client_line(&mut self, line: &str) -> Forward {
        if let Ok(request) = serde_json::from_str::<Value>(line) {
            if request["method"] == "initialize" {
                self.initialize = Some(request.clone());
            }
            if let Some(id) = request.get("id") {
                self.pending.push(id.clone());
            }
        }
        Forward::Server(line.into())
    }
    pub fn on_server_line(&mut self, line: &str) -> Option<String> {
        if let Ok(response) = serde_json::from_str::<Value>(line)
            && let Some(id) = response.get("id")
        {
            if self.replaying && id == "qapture-replay" {
                self.replaying = false;
                return None;
            }
            if let Some(i) = self.pending.iter().position(|pending| pending == id) {
                self.pending.remove(i);
            }
        }
        Some(line.into())
    }
    pub fn on_reconnect(&mut self) -> Vec<String> {
        let Some(mut initialize) = self.initialize.clone() else {
            return vec![];
        };
        initialize["id"] = json!("qapture-replay");
        self.replaying = true;
        vec![
            initialize.to_string(),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}).to_string(),
        ]
    }
    pub fn pending_errors(&mut self) -> Vec<String> {
        self.pending.drain(..).map(|id| json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":"Qapture se reinició; vuelve a intentar."}],"isError":true}}).to_string()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reconnect_replays_initialize_and_swallows_its_response() {
        let mut relay = Relay::default();
        relay.on_client_line(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientInfo":{"name":"test"}}}"#);
        relay.on_client_line(r#"{"jsonrpc":"2.0","id":2,"method":"tools/call"}"#);
        relay.on_server_line(r#"{"jsonrpc":"2.0","id":1,"result":{}}"#);
        let errors = relay.pending_errors();
        assert_eq!(errors.len(), 1);
        assert_eq!(serde_json::from_str::<Value>(&errors[0]).unwrap()["id"], 2);
        let replay = relay.on_reconnect();
        assert_eq!(replay.len(), 2);
        assert_eq!(
            serde_json::from_str::<Value>(&replay[0]).unwrap()["id"],
            "qapture-replay"
        );
        assert!(
            relay
                .on_server_line(r#"{"jsonrpc":"2.0","id":"qapture-replay","result":{}}"#)
                .is_none()
        );
    }
}
