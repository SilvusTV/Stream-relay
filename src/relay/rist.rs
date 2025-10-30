use anyhow::Result;

#[derive(Debug, Clone, Copy)]
pub enum RistMode {
    Listener,
    Caller,
}

#[derive(Debug, Clone, Copy)]
pub enum RistState {
    Init,
    Listening,
    Connected,
    Closed,
}

#[derive(Debug, Clone)]
pub struct RistEndpoint {
    input: String,
    output: String,
    mode: RistMode,
    state: RistState,
}

impl RistEndpoint {
    pub fn new(input: String, output: String) -> Self {
        let mode = infer_mode_from_uris(&input, &output);
        Self {
            input,
            output,
            mode,
            state: RistState::Init,
        }
    }

    pub fn open(&mut self) -> Result<()> {
        self.state = match self.mode {
            RistMode::Listener => RistState::Listening,
            RistMode::Caller => RistState::Connected,
        };
        Ok(())
    }

    pub fn close(&mut self) {
        self.state = RistState::Closed;
    }

    pub fn describe(&self) -> String {
        let mode = match self.mode { RistMode::Listener => "listener", RistMode::Caller => "caller" };
        let state = match self.state {
            RistState::Init => "INIT",
            RistState::Listening => "LISTENING",
            RistState::Connected => "CONNECTED",
            RistState::Closed => "CLOSED",
        };
        format!(
            "[RIST] input={} output={} mode={} state={}",
            normalize_uri(&self.input),
            normalize_uri(&self.output),
            mode,
            state,
        )
    }
}

fn infer_mode_from_uris(input: &str, _output: &str) -> RistMode {
    // Règle simple similaire au SRT: présence de sémantique "@" => listener
    if input.contains("rist://@") || uri_has_mode(input, "listener") {
        RistMode::Listener
    } else {
        RistMode::Caller
    }
}

fn uri_has_mode(uri: &str, mode: &str) -> bool {
    uri.split('?')
        .nth(1)
        .map(|q| q.split('&').any(|kv| kv.eq_ignore_ascii_case(&format!("mode={}", mode))))
        .unwrap_or(false)
}

fn normalize_uri(uri: &str) -> String {
    uri.to_string()
}
