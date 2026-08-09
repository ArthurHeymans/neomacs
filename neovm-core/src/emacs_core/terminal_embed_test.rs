use super::eval::{
    Context, DisplayHost, GuiFrameHostRequest, TerminalCreateRequest, TerminalDisplayMode,
    TerminalFloatPlacement,
};
use super::value::{Value, list_to_vec};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq)]
enum TerminalHostEvent {
    Create(TerminalCreateRequest),
    Write {
        id: u32,
        data: Vec<u8>,
    },
    Resize {
        id: u32,
        cols: u16,
        rows: u16,
    },
    Float {
        id: u32,
        placement: TerminalFloatPlacement,
    },
    Destroy {
        id: u32,
    },
}

#[derive(Clone, Default)]
struct RecordingTerminalDisplayHost {
    events: Arc<Mutex<Vec<TerminalHostEvent>>>,
}

impl DisplayHost for RecordingTerminalDisplayHost {
    fn realize_gui_frame(&mut self, _request: GuiFrameHostRequest) -> Result<(), String> {
        Ok(())
    }

    fn resize_gui_frame(&mut self, _request: GuiFrameHostRequest) -> Result<(), String> {
        Ok(())
    }

    fn create_terminal(&self, request: TerminalCreateRequest) -> Result<u32, String> {
        self.events
            .lock()
            .expect("terminal host events")
            .push(TerminalHostEvent::Create(request));
        Ok(41)
    }

    fn write_terminal(&self, id: u32, data: Vec<u8>) -> Result<(), String> {
        self.events
            .lock()
            .expect("terminal host events")
            .push(TerminalHostEvent::Write { id, data });
        Ok(())
    }

    fn resize_terminal(&self, id: u32, cols: u16, rows: u16) -> Result<(), String> {
        self.events
            .lock()
            .expect("terminal host events")
            .push(TerminalHostEvent::Resize { id, cols, rows });
        Ok(())
    }

    fn set_floating_terminal(
        &self,
        id: u32,
        placement: TerminalFloatPlacement,
    ) -> Result<(), String> {
        self.events
            .lock()
            .expect("terminal host events")
            .push(TerminalHostEvent::Float { id, placement });
        Ok(())
    }

    fn destroy_terminal(&self, id: u32) -> Result<(), String> {
        self.events
            .lock()
            .expect("terminal host events")
            .push(TerminalHostEvent::Destroy { id });
        Ok(())
    }

    fn terminal_text(&self, id: u32) -> Result<Option<String>, String> {
        Ok((id == 41).then(|| "ready\n$".to_owned()))
    }
}

#[test]
fn public_terminal_builtins_route_typed_requests_through_the_display_host() {
    crate::test_utils::init_test_tracing();
    let host = RecordingTerminalDisplayHost::default();
    let mut eval = Context::new();
    eval.set_display_host(Box::new(host.clone()));

    let result = eval
        .eval_str(
            r#"
(list
 (mapcar #'fboundp
         '(neomacs-terminal-create
           neomacs-terminal-write
           neomacs-terminal-resize
           neomacs-terminal-destroy
           neomacs-terminal-set-float
           neomacs-terminal-get-text))
 (let ((id (neomacs-terminal-create 80 24 2 "/bin/sh")))
   (list id
         (neomacs-terminal-write id "echo ready\r")
         (neomacs-terminal-resize id 120 40)
         (neomacs-terminal-set-float id 10.5 20 0.85)
         (neomacs-terminal-get-text id)
         (neomacs-terminal-destroy id)
         (neomacs-terminal-get-text 999))))
"#,
        )
        .expect("terminal public workflow should evaluate");

    let outer = list_to_vec(&result).expect("outer result list");
    assert_eq!(
        list_to_vec(&outer[0]).expect("fboundp result list"),
        vec![Value::T; 6]
    );
    let values = list_to_vec(&outer[1]).expect("workflow result list");
    assert_eq!(values[0], Value::fixnum(41));
    assert_eq!(&values[1..4], &[Value::T, Value::T, Value::T]);
    assert_eq!(values[4].as_utf8_str(), Some("ready\n$"));
    assert_eq!(values[5], Value::T);
    assert_eq!(values[6], Value::NIL);

    assert_eq!(
        *host.events.lock().expect("terminal host events"),
        vec![
            TerminalHostEvent::Create(TerminalCreateRequest {
                cols: 80,
                rows: 24,
                mode: TerminalDisplayMode::Floating,
                shell: Some("/bin/sh".to_owned()),
            }),
            TerminalHostEvent::Write {
                id: 41,
                data: b"echo ready\r".to_vec(),
            },
            TerminalHostEvent::Resize {
                id: 41,
                cols: 120,
                rows: 40,
            },
            TerminalHostEvent::Float {
                id: 41,
                placement: TerminalFloatPlacement {
                    x: 10.5,
                    y: 20.0,
                    opacity: 0.85,
                },
            },
            TerminalHostEvent::Destroy { id: 41 },
        ]
    );
}
