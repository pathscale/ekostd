pub enum ControlFlow<B, C = ()> {
    #[lang = "Continue"]
    Continue(C),
    #[lang = "Break"]
    Break(B),
}

impl<B, C> ControlFlow<B, C> {
    pub fn is_break(&self) -> bool { match self { ControlFlow::Break(_) => true, ControlFlow::Continue(_) => false } }
    pub fn is_continue(&self) -> bool { match self { ControlFlow::Continue(_) => true, ControlFlow::Break(_) => false } }
}

const impl<B, C> Try for ControlFlow<B, C> {
    type Output = C;
    type Residual = ControlFlow<B, crate::convert::Infallible>;
    fn from_output(output: C) -> Self { ControlFlow::Continue(output) }
    fn branch(self) -> ControlFlow<Self::Residual, C> {
        match self {
            ControlFlow::Continue(c) => ControlFlow::Continue(c),
            ControlFlow::Break(b) => ControlFlow::Break(ControlFlow::Break(b)),
        }
    }
}

const impl<B, C> FromResidual<ControlFlow<B, crate::convert::Infallible>> for ControlFlow<B, C> {
    fn from_residual(residual: ControlFlow<B, crate::convert::Infallible>) -> Self {
        match residual { ControlFlow::Break(b) => ControlFlow::Break(b), ControlFlow::Continue(never) => match never {} }
    }
}

#[lang = "Try"]
pub const trait Try: [const] FromResidual {
    type Output;
    type Residual;

    #[lang = "from_output"]
    fn from_output(output: Self::Output) -> Self;

    #[lang = "branch"]
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output>;
}

#[rustc_diagnostic_item = "FromResidual"]
pub const trait FromResidual<R = <Self as Try>::Residual> {
    #[lang = "from_residual"]
    fn from_residual(residual: R) -> Self;
}
