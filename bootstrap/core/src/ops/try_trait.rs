pub enum ControlFlow<B, C = ()> {
    #[lang = "Continue"]
    Continue(C),
    #[lang = "Break"]
    Break(B),
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
