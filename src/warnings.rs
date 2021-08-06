pub struct Warned<T> {
    pub value: T,
    pub warnings: Vec<String>,
}

impl<T> From<T> for Warned<T> {
    fn from(value: T) -> Self {
        Warned {
            value,
            warnings: vec![],
        }
    }
}

impl<T, I: Into<String>> From<(T, I)> for Warned<T> {
    fn from((value, warn): (T, I)) -> Self {
        Warned {
            value,
            warnings: vec![warn.into()],
        }
    }
}

impl<T> From<Warned<T>> for (T, Vec<String>) {
    fn from(w: Warned<T>) -> Self {
        (w.value, w.warnings)
    }
}

pub struct WarnContext {
    warnings: Vec<String>,
}

impl WarnContext {
    pub fn run<T, E>(action: impl Fn(&mut Self) -> Result<T, E>) -> Result<Warned<T>, E> {
        let mut wc = Self { warnings: vec![] };
        let t = action(&mut wc)?;
        Ok(Warned { value: t, warnings: wc.warnings })
    }
}

pub trait Unwarn {
    type Value;
    fn unwarn(self, wc: &mut WarnContext) -> Self::Value;
}

impl<T> Unwarn for Warned<T> {
    type Value = T;
    fn unwarn(self, wc: &mut WarnContext) -> Self::Value {
        let mut warnings = self.warnings.clone();
        wc.warnings.append(&mut warnings);
        self.value
    }
}

impl<T, E> Unwarn for Result<Warned<T>, E> {
    type Value = Result<T, E>;
    fn unwarn(self, wc: &mut WarnContext) -> Self::Value {
        match self {
            Err(e) => Err(e),
            Ok(t) => {
                let mut warnings = t.warnings.clone();
                wc.warnings.append(&mut warnings);
                Ok(t.value)
            }
        }
    }
}
