#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    Pure,
    ReadLexical,
    ReadSymbol,
    WriteSymbol,
    BindDynamic,
    Allocate,
    Call,
    MayGc,
    MaySignal,
    MayThrow,
    MayQuit,
    MayReenterElisp,
    BlockingIo,
    Unknown,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Effects {
    effects: Vec<Effect>,
}

impl Effects {
    pub fn pure() -> Self {
        Self {
            effects: vec![Effect::Pure],
        }
    }

    pub fn conservative_call() -> Self {
        Self {
            effects: vec![
                Effect::Call,
                Effect::MayGc,
                Effect::MaySignal,
                Effect::MayThrow,
                Effect::MayQuit,
                Effect::MayReenterElisp,
            ],
        }
    }

    pub fn contains(&self, effect: Effect) -> bool {
        self.effects.contains(&effect)
    }

    pub fn as_slice(&self) -> &[Effect] {
        &self.effects
    }
}
