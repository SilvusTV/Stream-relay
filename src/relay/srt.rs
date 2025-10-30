use anyhow::Result;

#[derive(Debug, Clone, Copy)]
pub enum SrtMode {
    Listener,
    Caller,
}

#[derive(Debug, Clone, Copy)]
pub enum SrtState {
    Init,
    Listening,
    Connected,
    Closed,
}

#[derive(Debug, Clone)]
pub struct SrtEndpoint {
    input: String,
    output: String,
    latency_ms: u64,
    mode: SrtMode,
    state: SrtState,
}

impl SrtEndpoint {
    pub fn new(input: String, output: String, latency_ms: u64) -> Self {
        let mode = infer_mode_from_uris(&input, &output);
        Self {
            input,
            output,
            latency_ms,
            mode,
            state: SrtState::Init,
        }
    }

    pub fn open(&mut self) -> Result<()> {
        // Ici on ouvrirait/attacherait le socket SRT (FFI). Pour ce smoke test, on simule.
        self.state = match self.mode {
            SrtMode::Listener => SrtState::Listening,
            SrtMode::Caller => SrtState::Connected,
        };
        Ok(())
    }

    pub fn close(&mut self) {
        self.state = SrtState::Closed;
    }

    pub fn describe(&self) -> String {
        let mode = match self.mode { SrtMode::Listener => "listener", SrtMode::Caller => "caller" };
        let state = match self.state {
            SrtState::Init => "INIT",
            SrtState::Listening => "LISTENING",
            SrtState::Connected => "CONNECTED",
            SrtState::Closed => "CLOSED",
        };
        format!(
            "[SRT] input={} output={} mode={} state={} latency_ms={}",
            normalize_uri(&self.input),
            normalize_uri(&self.output),
            mode,
            state,
            self.latency_ms
        )
    }
}

fn infer_mode_from_uris(input: &str, output: &str) -> SrtMode {
    // Règles simples: si input contient mode=listener ou host '@' => listener, sinon caller
    if uri_has_mode(input, "listener") || input.contains("srt://@") {
        SrtMode::Listener
    } else if uri_has_mode(output, "caller") {
        SrtMode::Caller
    } else {
        // fallback basique: si output a "mode=caller" ou host explicite, considérer Caller
        SrtMode::Caller
    }
}

fn uri_has_mode(uri: &str, mode: &str) -> bool {
    uri.split('?')
        .nth(1)
        .map(|q| q.split('&').any(|kv| kv.eq_ignore_ascii_case(&format!("mode={}", mode))))
        .unwrap_or(false)
}

fn normalize_uri(uri: &str) -> String {
    // Pour l'instant, renvoyer tel quel (placeholder). Une vraie normalisation pourrait trier les query params, etc.
    uri.to_string()
}
