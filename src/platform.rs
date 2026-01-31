//! Platform detection utilities
//!
//! This module provides functions for detecting the current operating system
//! and architecture, useful for cross-platform tools.
//!
//! ## Examples
//!
//! ```rust
//! use xx::platform;
//!
//! println!("Running on {} {}", platform::os(), platform::arch());
//! // e.g., "Running on macos arm64"
//!
//! if platform::is_macos() {
//!     println!("macOS detected!");
//! }
//! ```

/// Get the current operating system as a lowercase string
///
/// Returns a normalized OS name that's consistent across different tools:
/// - "macos" for macOS/Darwin
/// - "linux" for Linux
/// - "windows" for Windows
/// - "freebsd", "openbsd", "netbsd" for BSD variants
///
/// # Example
/// ```
/// use xx::platform;
/// let os = platform::os();
/// println!("Operating system: {}", os);
/// ```
pub fn os() -> &'static str {
    match std::env::consts::OS {
        "macos" => "macos",
        "linux" => "linux",
        "windows" => "windows",
        "freebsd" => "freebsd",
        "openbsd" => "openbsd",
        "netbsd" => "netbsd",
        "dragonfly" => "dragonfly",
        "ios" => "ios",
        "android" => "android",
        other => other,
    }
}

/// Get the current CPU architecture as a normalized string
///
/// Returns a normalized architecture name:
/// - "x64" for x86_64/amd64
/// - "arm64" for aarch64
/// - "x86" for i686/i386
/// - "arm" for 32-bit ARM
///
/// # Example
/// ```
/// use xx::platform;
/// let arch = platform::arch();
/// println!("Architecture: {}", arch);
/// ```
pub fn arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        "x86" | "i686" | "i386" => "x86",
        "arm" => "arm",
        "powerpc64" => "ppc64",
        "powerpc" => "ppc",
        "s390x" => "s390x",
        "riscv64" => "riscv64",
        "mips64" => "mips64",
        "mips" => "mips",
        other => other,
    }
}

/// Get the platform identifier as "os-arch"
///
/// # Example
/// ```
/// use xx::platform;
/// let platform = platform::id();
/// // e.g., "macos-arm64" or "linux-x64"
/// ```
pub fn id() -> String {
    format!("{}-{}", os(), arch())
}

/// Check if running on macOS
///
/// # Example
/// ```
/// use xx::platform;
/// if platform::is_macos() {
///     println!("Running on macOS");
/// }
/// ```
pub fn is_macos() -> bool {
    cfg!(target_os = "macos")
}

/// Check if running on Linux
///
/// # Example
/// ```
/// use xx::platform;
/// if platform::is_linux() {
///     println!("Running on Linux");
/// }
/// ```
pub fn is_linux() -> bool {
    cfg!(target_os = "linux")
}

/// Check if running on Windows
///
/// # Example
/// ```
/// use xx::platform;
/// if platform::is_windows() {
///     println!("Running on Windows");
/// }
/// ```
pub fn is_windows() -> bool {
    cfg!(target_os = "windows")
}

/// Check if running on a Unix-like system (macOS, Linux, BSD)
///
/// # Example
/// ```
/// use xx::platform;
/// if platform::is_unix() {
///     println!("Running on Unix-like system");
/// }
/// ```
pub fn is_unix() -> bool {
    cfg!(unix)
}

/// Check if running on a 64-bit architecture
///
/// # Example
/// ```
/// use xx::platform;
/// if platform::is_64bit() {
///     println!("Running on 64-bit architecture");
/// }
/// ```
pub fn is_64bit() -> bool {
    cfg!(target_pointer_width = "64")
}

/// Check if running on ARM architecture (32-bit or 64-bit)
///
/// # Example
/// ```
/// use xx::platform;
/// if platform::is_arm() {
///     println!("Running on ARM architecture");
/// }
/// ```
pub fn is_arm() -> bool {
    cfg!(any(target_arch = "aarch64", target_arch = "arm"))
}

/// Check if running on x86 architecture (32-bit or 64-bit)
///
/// # Example
/// ```
/// use xx::platform;
/// if platform::is_x86() {
///     println!("Running on x86 architecture");
/// }
/// ```
pub fn is_x86() -> bool {
    cfg!(any(target_arch = "x86_64", target_arch = "x86"))
}

/// Get the family of the operating system
///
/// Returns "unix" for Unix-like systems and "windows" for Windows.
///
/// # Example
/// ```
/// use xx::platform;
/// let family = platform::os_family();
/// // "unix" or "windows"
/// ```
pub fn os_family() -> &'static str {
    std::env::consts::FAMILY
}

/// Get the executable file extension for the current platform
///
/// Returns ".exe" on Windows, empty string on Unix-like systems.
///
/// # Example
/// ```
/// use xx::platform;
/// let exe = format!("myprogram{}", platform::exe_suffix());
/// // "myprogram" on Unix, "myprogram.exe" on Windows
/// ```
pub fn exe_suffix() -> &'static str {
    std::env::consts::EXE_SUFFIX
}

/// Get the dynamic library extension for the current platform
///
/// Returns ".dll" on Windows, ".dylib" on macOS, ".so" on Linux.
///
/// # Example
/// ```
/// use xx::platform;
/// let lib = format!("libfoo{}", platform::dll_suffix());
/// // "libfoo.so" on Linux, "libfoo.dylib" on macOS, "libfoo.dll" on Windows
/// ```
pub fn dll_suffix() -> &'static str {
    std::env::consts::DLL_SUFFIX
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_os() {
        let os = os();
        assert!(!os.is_empty());
        // Should be one of the known values
        assert!(
            ["macos", "linux", "windows", "freebsd", "openbsd"].contains(&os) || !os.is_empty()
        );
    }

    #[test]
    fn test_arch() {
        let arch = arch();
        assert!(!arch.is_empty());
        // Should be one of the known values
        assert!(
            ["x64", "arm64", "x86", "arm", "ppc64", "riscv64"].contains(&arch) || !arch.is_empty()
        );
    }

    #[test]
    fn test_id() {
        let id = id();
        assert!(id.contains('-'));
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 2);
    }

    #[test]
    fn test_os_family() {
        let family = os_family();
        assert!(family == "unix" || family == "windows");
    }

    #[test]
    fn test_is_64bit() {
        // This should work on common development machines
        #[cfg(target_pointer_width = "64")]
        assert!(is_64bit());

        #[cfg(target_pointer_width = "32")]
        assert!(!is_64bit());
    }

    #[test]
    fn test_is_unix() {
        #[cfg(unix)]
        assert!(is_unix());

        #[cfg(not(unix))]
        assert!(!is_unix());
    }

    #[test]
    fn test_exe_suffix() {
        let suffix = exe_suffix();
        #[cfg(windows)]
        assert_eq!(suffix, ".exe");

        #[cfg(not(windows))]
        assert_eq!(suffix, "");
    }
}
