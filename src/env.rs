//! Environment variable utilities
//!
//! This module provides convenient functions for working with environment variables,
//! including parsing boolean values, paths, numeric types, and more.
//!
//! ## Examples
//!
//! ```rust
//! use xx::env;
//!
//! // Check if an environment variable is truthy
//! unsafe { std::env::set_var("DEBUG", "1"); }
//! assert!(env::var_is_true("DEBUG"));
//!
//! // Parse a path with tilde expansion
//! unsafe { std::env::set_var("CONFIG_PATH", "~/config"); }
//! let path = env::var_path("CONFIG_PATH");
//!
//! // Parse numeric values
//! unsafe { std::env::set_var("THREADS", "4"); }
//! let threads = env::var_u32("THREADS").unwrap_or(1);
//!
//! // Parse comma-separated values
//! unsafe { std::env::set_var("TAGS", "a,b,c"); }
//! let tags = env::var_csv("TAGS");
//! assert_eq!(tags, vec!["a", "b", "c"]);
//!
//! // Parse durations
//! unsafe { std::env::set_var("TIMEOUT", "30s"); }
//! let timeout = env::var_duration("TIMEOUT");
//! ```

use std::env;
use std::path::PathBuf;
use std::time::Duration;

/// Check if an environment variable is set to a truthy value
///
/// Truthy values: "1", "true", "yes", "on" (case-insensitive)
///
/// # Example
/// ```
/// use xx::env;
/// unsafe { std::env::set_var("ENABLED", "true"); }
/// assert!(env::var_is_true("ENABLED"));
/// ```
pub fn var_is_true<K: AsRef<str>>(key: K) -> bool {
    match env::var(key.as_ref()) {
        Ok(val) => {
            let val = val.to_lowercase();
            val == "1" || val == "true" || val == "yes" || val == "on"
        }
        Err(_) => false,
    }
}

/// Check if an environment variable is set to a falsy value
///
/// Falsy values: "0", "false", "no", "off" (case-insensitive)
///
/// # Example
/// ```
/// use xx::env;
/// unsafe { std::env::set_var("DISABLED", "false"); }
/// assert!(env::var_is_false("DISABLED"));
/// ```
pub fn var_is_false<K: AsRef<str>>(key: K) -> bool {
    match env::var(key.as_ref()) {
        Ok(val) => {
            let val = val.to_lowercase();
            val == "0" || val == "false" || val == "no" || val == "off"
        }
        Err(_) => false,
    }
}

/// Parse an environment variable as an optional boolean
///
/// Returns Some(true) for truthy values, Some(false) for falsy values,
/// and None if the variable is not set or has an unrecognized value.
///
/// # Example
/// ```
/// use xx::env;
/// unsafe { std::env::set_var("FEATURE", "yes"); }
/// assert_eq!(env::var_option_bool("FEATURE"), Some(true));
/// ```
pub fn var_option_bool<K: AsRef<str>>(key: K) -> Option<bool> {
    let key = key.as_ref();
    if var_is_true(key) {
        Some(true)
    } else if var_is_false(key) {
        Some(false)
    } else {
        None
    }
}

/// Parse an environment variable as a path with tilde expansion
///
/// Expands "~" to the home directory if present at the start of the path.
///
/// # Example
/// ```
/// use xx::env;
/// unsafe { std::env::set_var("MY_PATH", "~/documents"); }
/// let path = env::var_path("MY_PATH");
/// // path will be Some(PathBuf) with expanded home directory
/// ```
pub fn var_path<K: AsRef<str>>(key: K) -> Option<PathBuf> {
    env::var(key.as_ref()).ok().map(|val| {
        if let Some(stripped) = val.strip_prefix("~/") {
            if let Some(home) = homedir::my_home().ok().flatten() {
                home.join(stripped)
            } else {
                PathBuf::from(val)
            }
        } else {
            PathBuf::from(val)
        }
    })
}

/// Parse an environment variable as a u8
///
/// # Example
/// ```
/// use xx::env;
/// unsafe { std::env::set_var("LEVEL", "5"); }
/// assert_eq!(env::var_u8("LEVEL"), Some(5));
/// ```
pub fn var_u8<K: AsRef<str>>(key: K) -> Option<u8> {
    env::var(key.as_ref()).ok().and_then(|val| val.parse().ok())
}

/// Parse an environment variable as a u32
///
/// # Example
/// ```
/// use xx::env;
/// unsafe { std::env::set_var("COUNT", "42"); }
/// assert_eq!(env::var_u32("COUNT"), Some(42));
/// ```
pub fn var_u32<K: AsRef<str>>(key: K) -> Option<u32> {
    env::var(key.as_ref()).ok().and_then(|val| val.parse().ok())
}

/// Parse an environment variable as a u64
///
/// # Example
/// ```
/// use xx::env;
/// unsafe { std::env::set_var("SIZE", "1000000"); }
/// assert_eq!(env::var_u64("SIZE"), Some(1000000));
/// ```
pub fn var_u64<K: AsRef<str>>(key: K) -> Option<u64> {
    env::var(key.as_ref()).ok().and_then(|val| val.parse().ok())
}

/// Parse an environment variable as an i32
///
/// # Example
/// ```
/// use xx::env;
/// unsafe { std::env::set_var("OFFSET", "-10"); }
/// assert_eq!(env::var_i32("OFFSET"), Some(-10));
/// ```
pub fn var_i32<K: AsRef<str>>(key: K) -> Option<i32> {
    env::var(key.as_ref()).ok().and_then(|val| val.parse().ok())
}

/// Parse an environment variable as an i64
///
/// # Example
/// ```
/// use xx::env;
/// unsafe { std::env::set_var("TIMESTAMP", "1234567890"); }
/// assert_eq!(env::var_i64("TIMESTAMP"), Some(1234567890));
/// ```
pub fn var_i64<K: AsRef<str>>(key: K) -> Option<i64> {
    env::var(key.as_ref()).ok().and_then(|val| val.parse().ok())
}

/// Parse an environment variable as a usize
///
/// # Example
/// ```
/// use xx::env;
/// unsafe { std::env::set_var("COUNT", "100"); }
/// assert_eq!(env::var_usize("COUNT"), Some(100));
/// ```
pub fn var_usize<K: AsRef<str>>(key: K) -> Option<usize> {
    env::var(key.as_ref()).ok().and_then(|val| val.parse().ok())
}

/// Parse an environment variable as comma-separated values
///
/// Returns an empty vector if the variable is not set.
/// Whitespace around values is trimmed.
///
/// # Example
/// ```
/// use xx::env;
/// unsafe { std::env::set_var("TAGS", "foo, bar, baz"); }
/// assert_eq!(env::var_csv("TAGS"), vec!["foo", "bar", "baz"]);
///
/// // Returns empty vec if not set
/// assert_eq!(env::var_csv("NONEXISTENT_CSV_VAR"), Vec::<String>::new());
/// ```
pub fn var_csv<K: AsRef<str>>(key: K) -> Vec<String> {
    env::var(key.as_ref())
        .ok()
        .map(|val| {
            val.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Parse an environment variable as a Duration
///
/// Supports formats like:
/// - "30" or "30s" - seconds
/// - "5m" - minutes
/// - "2h" - hours
/// - "1d" - days
/// - "100ms" - milliseconds
///
/// # Example
/// ```
/// use xx::env;
/// use std::time::Duration;
///
/// unsafe { std::env::set_var("TIMEOUT", "30s"); }
/// assert_eq!(env::var_duration("TIMEOUT"), Some(Duration::from_secs(30)));
///
/// unsafe { std::env::set_var("INTERVAL", "5m"); }
/// assert_eq!(env::var_duration("INTERVAL"), Some(Duration::from_secs(300)));
///
/// unsafe { std::env::set_var("DELAY", "100ms"); }
/// assert_eq!(env::var_duration("DELAY"), Some(Duration::from_millis(100)));
/// ```
pub fn var_duration<K: AsRef<str>>(key: K) -> Option<Duration> {
    env::var(key.as_ref())
        .ok()
        .and_then(|val| parse_duration(&val))
}

/// Parse a duration string into a Duration
///
/// Supports formats like "30s", "5m", "2h", "1d", "100ms"
pub fn parse_duration(s: &str) -> Option<Duration> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    // Try to find where the number ends and the unit begins
    let (num_str, unit) = if let Some(stripped) = s.strip_suffix("ms") {
        (stripped, "ms")
    } else if let Some(stripped) = s.strip_suffix('s') {
        (stripped, "s")
    } else if let Some(stripped) = s.strip_suffix('m') {
        (stripped, "m")
    } else if let Some(stripped) = s.strip_suffix('h') {
        (stripped, "h")
    } else if let Some(stripped) = s.strip_suffix('d') {
        (stripped, "d")
    } else {
        // Assume seconds if no unit
        (s, "s")
    };

    let num: u64 = num_str.trim().parse().ok()?;

    Some(match unit {
        "ms" => Duration::from_millis(num),
        "s" => Duration::from_secs(num),
        "m" => Duration::from_secs(num * 60),
        "h" => Duration::from_secs(num * 60 * 60),
        "d" => Duration::from_secs(num * 60 * 60 * 24),
        _ => return None,
    })
}

/// Parse an environment variable as a log level
///
/// Supported values (case-insensitive):
/// - "trace", "5"
/// - "debug", "4"
/// - "info", "3"
/// - "warn", "warning", "2"
/// - "error", "1"
/// - "off", "0", "none"
///
/// # Example
/// ```
/// use xx::env;
/// use log::LevelFilter;
///
/// unsafe { std::env::set_var("LOG_LEVEL", "debug"); }
/// assert_eq!(env::var_log_level("LOG_LEVEL"), Some(LevelFilter::Debug));
///
/// unsafe { std::env::set_var("LOG_LEVEL", "warn"); }
/// assert_eq!(env::var_log_level("LOG_LEVEL"), Some(LevelFilter::Warn));
/// ```
pub fn var_log_level<K: AsRef<str>>(key: K) -> Option<log::LevelFilter> {
    env::var(key.as_ref())
        .ok()
        .and_then(|val| parse_log_level(&val))
}

/// Parse a string into a log level
pub fn parse_log_level(s: &str) -> Option<log::LevelFilter> {
    let s = s.trim().to_lowercase();
    Some(match s.as_str() {
        "trace" | "5" => log::LevelFilter::Trace,
        "debug" | "4" => log::LevelFilter::Debug,
        "info" | "3" => log::LevelFilter::Info,
        "warn" | "warning" | "2" => log::LevelFilter::Warn,
        "error" | "1" => log::LevelFilter::Error,
        "off" | "0" | "none" => log::LevelFilter::Off,
        _ => return None,
    })
}

/// Get an environment variable or return a default value
///
/// # Example
/// ```
/// use xx::env;
///
/// // Returns default if not set
/// let val = env::var_or("NONEXISTENT_VAR", "default");
/// assert_eq!(val, "default");
///
/// // Returns value if set
/// unsafe { std::env::set_var("MY_VAR", "custom"); }
/// let val = env::var_or("MY_VAR", "default");
/// assert_eq!(val, "custom");
/// ```
pub fn var_or<K: AsRef<str>>(key: K, default: &str) -> String {
    env::var(key.as_ref()).unwrap_or_else(|_| default.to_string())
}

/// Get an environment variable as a PathBuf or return a default
///
/// Supports tilde expansion for home directory.
///
/// # Example
/// ```
/// use xx::env;
/// use std::path::PathBuf;
///
/// let path = env::var_path_or("NONEXISTENT_PATH", "~/.config");
/// // Returns expanded default path
/// ```
pub fn var_path_or<K: AsRef<str>>(key: K, default: &str) -> PathBuf {
    var_path(key).unwrap_or_else(|| {
        if let Some(stripped) = default.strip_prefix("~/")
            && let Some(home) = homedir::my_home().ok().flatten()
        {
            return home.join(stripped);
        }
        PathBuf::from(default)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Mutex to ensure tests don't interfere with each other
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_var_is_true() {
        let _guard = TEST_MUTEX.lock().unwrap();

        unsafe {
            env::set_var("TEST_TRUE", "1");
        }
        assert!(var_is_true("TEST_TRUE"));

        unsafe {
            env::set_var("TEST_TRUE", "true");
        }
        assert!(var_is_true("TEST_TRUE"));

        unsafe {
            env::set_var("TEST_TRUE", "YES");
        }
        assert!(var_is_true("TEST_TRUE"));

        unsafe {
            env::set_var("TEST_TRUE", "on");
        }
        assert!(var_is_true("TEST_TRUE"));

        unsafe {
            env::set_var("TEST_TRUE", "false");
        }
        assert!(!var_is_true("TEST_TRUE"));

        assert!(!var_is_true("NONEXISTENT_VAR_TRUE"));
    }

    #[test]
    fn test_var_is_false() {
        let _guard = TEST_MUTEX.lock().unwrap();

        unsafe {
            env::set_var("TEST_FALSE", "0");
        }
        assert!(var_is_false("TEST_FALSE"));

        unsafe {
            env::set_var("TEST_FALSE", "false");
        }
        assert!(var_is_false("TEST_FALSE"));

        unsafe {
            env::set_var("TEST_FALSE", "NO");
        }
        assert!(var_is_false("TEST_FALSE"));

        unsafe {
            env::set_var("TEST_FALSE", "off");
        }
        assert!(var_is_false("TEST_FALSE"));

        unsafe {
            env::set_var("TEST_FALSE", "true");
        }
        assert!(!var_is_false("TEST_FALSE"));

        assert!(!var_is_false("NONEXISTENT_VAR_FALSE"));
    }

    #[test]
    fn test_var_option_bool() {
        let _guard = TEST_MUTEX.lock().unwrap();

        unsafe {
            env::set_var("TEST_BOOL", "true");
        }
        assert_eq!(var_option_bool("TEST_BOOL"), Some(true));

        unsafe {
            env::set_var("TEST_BOOL", "false");
        }
        assert_eq!(var_option_bool("TEST_BOOL"), Some(false));

        unsafe {
            env::set_var("TEST_BOOL", "maybe");
        }
        assert_eq!(var_option_bool("TEST_BOOL"), None);

        assert_eq!(var_option_bool("NONEXISTENT_VAR_BOOL"), None);
    }

    #[test]
    fn test_var_numeric() {
        let _guard = TEST_MUTEX.lock().unwrap();

        unsafe {
            env::set_var("TEST_U8", "255");
        }
        assert_eq!(var_u8("TEST_U8"), Some(255));

        unsafe {
            env::set_var("TEST_U32", "42");
        }
        assert_eq!(var_u32("TEST_U32"), Some(42));

        unsafe {
            env::set_var("TEST_U64", "1000000");
        }
        assert_eq!(var_u64("TEST_U64"), Some(1000000));

        unsafe {
            env::set_var("TEST_I32", "-100");
        }
        assert_eq!(var_i32("TEST_I32"), Some(-100));

        unsafe {
            env::set_var("TEST_I64", "9223372036854775807");
        }
        assert_eq!(var_i64("TEST_I64"), Some(9223372036854775807));

        unsafe {
            env::set_var("TEST_INVALID", "not_a_number");
        }
        assert_eq!(var_u32("TEST_INVALID"), None);
    }

    #[test]
    fn test_var_csv() {
        let _guard = TEST_MUTEX.lock().unwrap();

        unsafe {
            env::set_var("TEST_CSV", "a, b, c");
        }
        assert_eq!(var_csv("TEST_CSV"), vec!["a", "b", "c"]);

        unsafe {
            env::set_var("TEST_CSV_SINGLE", "single");
        }
        assert_eq!(var_csv("TEST_CSV_SINGLE"), vec!["single"]);

        unsafe {
            env::set_var("TEST_CSV_EMPTY", "");
        }
        assert!(var_csv("TEST_CSV_EMPTY").is_empty());

        assert!(var_csv("NONEXISTENT_CSV_VAR").is_empty());
    }

    #[test]
    fn test_var_duration() {
        let _guard = TEST_MUTEX.lock().unwrap();
        use std::time::Duration;

        unsafe {
            env::set_var("TEST_DUR_SEC", "30s");
        }
        assert_eq!(var_duration("TEST_DUR_SEC"), Some(Duration::from_secs(30)));

        unsafe {
            env::set_var("TEST_DUR_MIN", "5m");
        }
        assert_eq!(var_duration("TEST_DUR_MIN"), Some(Duration::from_secs(300)));

        unsafe {
            env::set_var("TEST_DUR_HOUR", "2h");
        }
        assert_eq!(
            var_duration("TEST_DUR_HOUR"),
            Some(Duration::from_secs(7200))
        );

        unsafe {
            env::set_var("TEST_DUR_DAY", "1d");
        }
        assert_eq!(
            var_duration("TEST_DUR_DAY"),
            Some(Duration::from_secs(86400))
        );

        unsafe {
            env::set_var("TEST_DUR_MS", "100ms");
        }
        assert_eq!(
            var_duration("TEST_DUR_MS"),
            Some(Duration::from_millis(100))
        );

        unsafe {
            env::set_var("TEST_DUR_BARE", "60");
        }
        assert_eq!(var_duration("TEST_DUR_BARE"), Some(Duration::from_secs(60)));

        assert_eq!(var_duration("NONEXISTENT_DUR_VAR"), None);
    }

    #[test]
    fn test_var_log_level() {
        let _guard = TEST_MUTEX.lock().unwrap();
        use log::LevelFilter;

        unsafe {
            env::set_var("TEST_LOG_TRACE", "trace");
        }
        assert_eq!(var_log_level("TEST_LOG_TRACE"), Some(LevelFilter::Trace));

        unsafe {
            env::set_var("TEST_LOG_DEBUG", "DEBUG");
        }
        assert_eq!(var_log_level("TEST_LOG_DEBUG"), Some(LevelFilter::Debug));

        unsafe {
            env::set_var("TEST_LOG_INFO", "info");
        }
        assert_eq!(var_log_level("TEST_LOG_INFO"), Some(LevelFilter::Info));

        unsafe {
            env::set_var("TEST_LOG_WARN", "warning");
        }
        assert_eq!(var_log_level("TEST_LOG_WARN"), Some(LevelFilter::Warn));

        unsafe {
            env::set_var("TEST_LOG_ERROR", "error");
        }
        assert_eq!(var_log_level("TEST_LOG_ERROR"), Some(LevelFilter::Error));

        unsafe {
            env::set_var("TEST_LOG_OFF", "off");
        }
        assert_eq!(var_log_level("TEST_LOG_OFF"), Some(LevelFilter::Off));

        unsafe {
            env::set_var("TEST_LOG_NUM", "4");
        }
        assert_eq!(var_log_level("TEST_LOG_NUM"), Some(LevelFilter::Debug));

        assert_eq!(var_log_level("NONEXISTENT_LOG_VAR"), None);
    }

    #[test]
    fn test_var_or() {
        let _guard = TEST_MUTEX.lock().unwrap();

        assert_eq!(var_or("NONEXISTENT_VAR_OR", "default"), "default");

        unsafe {
            env::set_var("TEST_VAR_OR", "custom");
        }
        assert_eq!(var_or("TEST_VAR_OR", "default"), "custom");
    }
}
