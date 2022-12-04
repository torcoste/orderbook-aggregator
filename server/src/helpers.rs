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
    let value = match env::var(var_name) {
        Ok(value) => {
            let value = value.parse::<T>().unwrap_or(default);
            value
        }
        Err(_) => default,
    };

    println!("{}: {}", var_name, value);
    value
}
