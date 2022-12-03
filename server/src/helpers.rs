use std::{
    env,
    fmt::{Debug, Display},
    str::FromStr,
};

pub fn get_env_var_or_default<T>(var_name: &str, default: T) -> T
where
    T: ToString + Display + Debug + FromStr,
    <T as FromStr>::Err: Debug,
{
    match env::var(var_name) {
        Ok(val) => T::from_str(&val).unwrap(),
        Err(error) => {
            eprintln!(
                "Error reading \"{}\" env var. Defaulting to \"{}\". Error details: {}",
                var_name, default, error
            );
            default
        }
    }
}
