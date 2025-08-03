use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tar::Archive;
use tempfile::TempDir;
use walkdir::WalkDir;
use zstd::stream::read::Decoder;
use infer::Infer;

#[derive(Parser)]
#[command(name = "arch-installer")]
#[command(about = "Utility for installing and uninstalling Arch Linux packages on any distribution")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Install {
        #[arg(value_name = "PACKAGE")]
        package: String,
        #[arg(long, default_value = "/usr/local")]
        prefix: String,
    },
    Uninstall {
        #[arg(value_name = "PACKAGE")]
        package: String,
        #[arg(long, default_value = "/usr/local")]
        prefix: String,
    },
    Reinstall {
        #[arg(value_name = "PACKAGE")]
        package: String,
        #[arg(long, default_value = "/usr/local")]
        prefix: String,
    },
    List,
    Info,
}

fn extract_pkg_zst(pkg_path: &str, temp_dir: &str) -> Result<()> {
    let file = File::open(pkg_path)
        .context(format!("Failed to open package {}", pkg_path))?;
    let decoder = Decoder::new(file)?;
    let mut archive = Archive::new(decoder);
    fs::create_dir_all(temp_dir)?;
    archive.unpack(temp_dir)
        .context("Error while extracting package")?;
    println!("Extracted package {} to {}", pkg_path, temp_dir);
    Ok(())
}

fn is_root() -> bool {
    #[cfg(unix)]
    {
        nix::unistd::geteuid().is_root()
    }
    #[cfg(not(unix))]
    {
        false
    }
}

fn get_package_name(pkg_path: &str) -> String {
    let file_name = Path::new(pkg_path)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or("unknown".to_string());
    file_name
        .split('-')
        .next()
        .map(|s| s.to_string())
        .unwrap_or("unknown".to_string())
}

fn get_user_home_dir() -> PathBuf {
    if let Ok(sudo_user) = std::env::var("SUDO_USER") {
        return PathBuf::from(format!("/home/{}", sudo_user));
    }
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"))
}

fn get_log_dir() -> PathBuf {
    get_user_home_dir().join(".local/share/arch-installer")
}

fn get_log_path(package: &str) -> PathBuf {
    get_log_dir().join(format!("{}.log", package))
}

fn parse_pkginfo(temp_dir: &str) -> Result<(Vec<String>, Vec<String>)> {
    let pkginfo_path = format!("{}/.PKGINFO", temp_dir);
    let content = fs::read_to_string(&pkginfo_path)
        .context(format!("Failed to read .PKGINFO from {}", pkginfo_path))?;
    let mut depends = Vec::new();
    let mut optdepends = Vec::new();
    for line in content.lines() {
        if line.starts_with("depend = ") {
            let depend = line.trim_start_matches("depend = ").trim().to_string();
            depends.push(depend);
        } else if line.starts_with("optdepend = ") {
            let optdepend = line.trim_start_matches("optdepend = ").trim().to_string();
            optdepends.push(optdepend);
        }
    }
    Ok((depends, optdepends))
}

fn confirm_installation(package: &str, depends: &[String], optdepends: &[String]) -> Result<bool> {
    println!("Package: {}", package);
    if depends.is_empty() {
        println!("No required dependencies listed.");
    } else {
        println!("Required dependencies:");
        for dep in depends {
            println!("  - {}", dep);
        }
    }
    if optdepends.is_empty() {
        println!("No optional dependencies listed.");
    } else {
        println!("Optional dependencies:");
        for optdep in optdepends {
            println!("  - {}", optdep);
        }
    }
    println!("Are you sure you want to install this package? [y/N]");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_lowercase() == "y")
}

fn confirm_uninstallation(package: &str) -> Result<bool> {
    println!("Are you sure you want to uninstall {}? [y/N]", package);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_lowercase() == "y")
}

fn clean_empty_dirs(path: &Path) -> Result<()> {
    if path.is_dir() {
        let is_empty = fs::read_dir(path)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false);
        if is_empty {
            fs::remove_dir(path)?;
            println!("Removed empty directory: {}", path.display());
            if let Some(parent) = path.parent() {
                clean_empty_dirs(parent)?;
            }
        }
    }
    Ok(())
}

fn list_packages() -> Result<()> {
    let log_dir = get_log_dir();
    if !log_dir.exists() {
        println!("0");
        return Ok(());
    }
    let count = fs::read_dir(&log_dir)?
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.extension().map(|ext| ext == "log").unwrap_or(false) {
                Some(())
            } else {
                None
            }
        })
        .count();
    println!("{}", count);
    Ok(())
}

fn get_system_info() -> Result<()> {
    let mut output = Vec::new();
    if let Ok(os_release) = fs::read_to_string("/etc/os-release") {
        let os = os_release
            .lines()
            .find(|line| line.starts_with("PRETTY_NAME="))
            .map(|line| line.trim_start_matches("PRETTY_NAME=\"").trim_end_matches("\""))
            .unwrap_or("Unknown");
        output.push(format!("OS: {}", os));
    } else {
        output.push("OS: Unknown".to_string());
    }
    if let Ok(kernel) = std::process::Command::new("uname").arg("-r").output() {
        let kernel = String::from_utf8_lossy(&kernel.stdout).trim().to_string();
        output.push(format!("Kernel: {}", kernel));
    } else {
        output.push("Kernel: Unknown".to_string());
    }
    if let Ok(shell) = std::env::var("SHELL") {
        let shell_name = Path::new(&shell).file_name().unwrap_or_default().to_string_lossy();
        if let Ok(version) = std::process::Command::new(&shell).arg("--version").output() {
            let version = String::from_utf8_lossy(&version.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .to_string();
            output.push(format!("Shell: {} {}", shell_name, version));
        } else {
            output.push(format!("Shell: {}", shell_name));
        }
    } else {
        output.push("Shell: Unknown".to_string());
    }
    let de = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "Unknown".to_string());
    output.push(format!("DE: {}", de));
    let mut packages = Vec::new();
    let log_dir = get_log_dir();
    let arch_installer_count = if log_dir.exists() {
        fs::read_dir(&log_dir)?
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                if path.extension().map(|ext| ext == "log").unwrap_or(false) {
                    Some(())
                } else {
                    None
                }
            })
            .count()
    } else {
        0
    };
    if arch_installer_count > 0 {
        packages.push(format!("arch-installer {}", arch_installer_count));
    }
    if Path::new("/usr/bin/pacman").exists() {
        if let Ok(output) = std::process::Command::new("pacman").arg("-Q").output() {
            let count = String::from_utf8_lossy(&output.stdout).lines().count();
            if count > 0 {
                packages.push(format!("pacman {}", count));
            }
        }
    }
    if Path::new("/usr/bin/dpkg").exists() {
        if let Ok(output) = std::process::Command::new("dpkg").arg("-l").output() {
            let count = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|line| line.starts_with("ii "))
                .count();
            if count > 0 {
                packages.push(format!("dpkg {}", count));
            }
        }
    }
    if Path::new("/usr/bin/rpm").exists() {
        if let Ok(output) = std::process::Command::new("rpm").arg("-qa").output() {
            let count = String::from_utf8_lossy(&output.stdout).lines().count();
            if count > 0 {
                packages.push(format!("rpm {}", count));
            }
        }
    }
    if packages.is_empty() {
        output.push("Packages: None".to_string());
    } else {
        output.push(format!("Packages: {}", packages.join(", ")));
    }
    for line in output {
        println!("{}", line);
    }
    Ok(())
}

fn install_files(temp_dir: &str, prefix: &str, package: &str) -> Result<()> {
    let needs_root = prefix.starts_with("/usr") || prefix == "/opt";
    if needs_root && !is_root() {
        anyhow::bail!("Please run the program with sudo or doas to install to {}", prefix);
    }
    let (depends, optdepends) = parse_pkginfo(temp_dir)?;
    if !confirm_installation(&get_package_name(package), &depends, &optdepends)? {
        anyhow::bail!("Installation cancelled by user.");
    }
    let src_bin_dir = format!("{}/usr/bin", temp_dir);
    let dest_bin_dir = format!("{}/bin", prefix);
    let src_desktop_dir = format!("{}/usr/share/applications", temp_dir);
    let dest_desktop_dir = if prefix == "/usr/local" {
        format!("{}/share/applications", prefix)
    } else {
        get_user_home_dir()
            .join(".local/share/applications")
            .to_string_lossy()
            .into_owned()
    };
    let src_icon_dir = format!("{}/usr/share/icons", temp_dir);
    let dest_icon_dir = if prefix == "/usr/local" {
        format!("{}/share/icons", prefix)
    } else {
        get_user_home_dir()
            .join(".local/share/icons")
            .to_string_lossy()
            .into_owned()
    };
    let package_name = get_package_name(package);
    let log_path = get_log_path(&package_name);
    fs::create_dir_all(log_path.parent().unwrap())?;
    let mut log_file = File::create(&log_path)
        .context(format!("Failed to create log file {}", log_path.display()))?;
    let infer = Infer::new();
    if Path::new(&src_bin_dir).exists() {
        fs::create_dir_all(&dest_bin_dir)?;
        for entry in WalkDir::new(&src_bin_dir).into_iter().filter_map(|e| e.ok()) {
            let src_path = entry.path();
            if src_path.is_file() {
                let file_content = fs::read(src_path)?;
                if let Some(kind) = infer.get(&file_content) {
                    if kind.mime_type().starts_with("application/x-executable") || kind.mime_type().starts_with("application/x-sharedlib") {
                        let relative_path = src_path.strip_prefix(&src_bin_dir)?;
                        let dest_path = Path::new(&dest_bin_dir).join(relative_path);
                        writeln!(log_file, "{}", dest_path.display())?;
                        if dest_path.exists() {
                            println!("Warning: file {} already exists, skipping", dest_path.display());
                            continue;
                        }
                        fs::copy(src_path, &dest_path)?;
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            fs::set_permissions(&dest_path, fs::Permissions::from_mode(0o755))?;
                        }
                        println!("Installed binary: {}", dest_path.display());
                    } else {
                        println!("Skipping non-ELF file: {}", src_path.display());
                    }
                } else {
                    println!("Skipping non-ELF file: {}", src_path.display());
                }
            }
        }
    } else {
        println!("No binaries found in /usr/bin, skipping");
    }
    if Path::new(&src_desktop_dir).exists() {
        fs::create_dir_all(&dest_desktop_dir)?;
        for entry in WalkDir::new(&src_desktop_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let src_path = entry.path();
            if src_path.is_file() && src_path.extension().map(|e| e == "desktop").unwrap_or(false) {
                let relative_path = src_path.strip_prefix(&src_desktop_dir)?;
                let dest_path = Path::new(&dest_desktop_dir).join(relative_path);
                writeln!(log_file, "{}", dest_path.display())?;
                if dest_path.exists() {
                    println!("Warning: file {} already exists, skipping", dest_path.display());
                    continue;
                }
                fs::copy(src_path, &dest_path)?;
                println!("Installed .desktop file: {}", dest_path.display());
            }
        }
        if prefix == "/usr/local" {
            if let Ok(output) = std::process::Command::new("update-desktop-database")
                .arg(&dest_desktop_dir)
                .output()
            {
                if !output.status.success() {
                    println!(
                        "Warning: failed to update desktop database: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                } else {
                    println!("Desktop database updated");
                }
            }
        }
    } else {
        println!("No .desktop files found, skipping");
    }
    if Path::new(&src_icon_dir).exists() {
        fs::create_dir_all(&dest_icon_dir)?;
        for entry in WalkDir::new(&src_icon_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let src_path = entry.path();
            if src_path.is_file() && src_path.extension().map(|e| e == "png" || e == "svg").unwrap_or(false) {
                let file_content = fs::read(src_path)?;
                let is_valid_icon = if let Some(kind) = infer.get(&file_content) {
                    kind.mime_type() == "image/png" || kind.mime_type() == "image/svg+xml"
                } else {
                    false
                };
                if is_valid_icon {
                    let relative_path = src_path.strip_prefix(&src_icon_dir)?;
                    let dest_path = Path::new(&dest_icon_dir).join(relative_path);
                    writeln!(log_file, "{}", dest_path.display())?;
                    if dest_path.exists() {
                        println!("Warning: icon {} already exists, skipping", dest_path.display());
                        continue;
                    }
                    fs::create_dir_all(dest_path.parent().unwrap())?;
                    fs::copy(src_path, &dest_path)?;
                    println!("Installed icon: {}", dest_path.display());
                } else {
                    println!("Skipping invalid icon: {}", src_path.display());
                }
            }
        }
    } else {
        println!("No icons found in /usr/share/icons, skipping");
    }
    Ok(())
}

fn uninstall_files(package: &str, prefix: &str) -> Result<()> {
    let needs_root = prefix.starts_with("/usr") || prefix == "/opt";
    if needs_root && !is_root() {
        anyhow::bail!("Please run the program with sudo or doas to uninstall from {}", prefix);
    }
    let package_name = get_package_name(package);
    if !confirm_uninstallation(&package_name)? {
        anyhow::bail!("Uninstallation cancelled by user.");
    }
    let log_path = get_log_path(&package_name);
    if !log_path.exists() {
        anyhow::bail!(
            "No installation log found for package {} at {}. Run install first.",
            package_name,
            log_path.display()
        );
    }
    let dest_bin_dir = format!("{}/bin", prefix);
    let dest_desktop_dir = if prefix == "/usr/local" {
        format!("{}/share/applications", prefix)
    } else {
        get_user_home_dir()
            .join(".local/share/applications")
            .to_string_lossy()
            .into_owned()
    };
    let dest_icon_dir = if prefix == "/usr/local" {
        format!("{}/share/icons", prefix)
    } else {
        get_user_home_dir()
            .join(".local/share/icons")
            .to_string_lossy()
            .into_owned()
    };
    let log_content = fs::read_to_string(&log_path)
        .context(format!("Failed to read log file {}", log_path.display()))?;
    for line in log_content.lines() {
        let file_path = Path::new(line);
        if file_path.exists() {
            fs::remove_file(file_path)
                .context(format!("Failed to remove file {}", file_path.display()))?;
            if file_path.extension().map(|e| e == "desktop").unwrap_or(false) {
                println!("Removed .desktop file: {}", file_path.display());
            } else if file_path.extension().map(|e| e == "png" || e == "svg").unwrap_or(false) {
                println!("Removed icon: {}", file_path.display());
            } else {
                println!("Removed file: {}", file_path.display());
            }
        } else {
            println!("File {} does not exist, skipping", file_path.display());
        }
    }
    fs::remove_file(&log_path)
        .context(format!("Failed to remove log file {}", log_path.display()))?;
    println!("Removed log file: {}", log_path.display());
    clean_empty_dirs(Path::new(&dest_bin_dir))?;
    clean_empty_dirs(Path::new(&dest_desktop_dir))?;
    clean_empty_dirs(Path::new(&dest_icon_dir))?;
    if prefix == "/usr/local" && Path::new(&dest_desktop_dir).exists() {
        if let Ok(output) = std::process::Command::new("update-desktop-database")
            .arg(&dest_desktop_dir)
            .output()
        {
            if !output.status.success() {
                println!(
                    "Warning: failed to update desktop database: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            } else {
                println!("Desktop database updated");
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Install { package, prefix } => {
            let temp_dir = TempDir::new()?.path().to_string_lossy().into_owned();
            extract_pkg_zst(&package, &temp_dir)?;
            install_files(&temp_dir, &prefix, &package)?;
            println!("Installation completed!");
            Ok(())
        }
        Commands::Uninstall { package, prefix } => {
            uninstall_files(&package, &prefix)?;
            println!("Uninstallation completed!");
            Ok(())
        }
        Commands::Reinstall { package, prefix } => {
            uninstall_files(&package, &prefix)?;
            let temp_dir = TempDir::new()?.path().to_string_lossy().into_owned();
            extract_pkg_zst(&package, &temp_dir)?;
            install_files(&temp_dir, &prefix, &package)?;
            println!("Reinstallation completed!");
            Ok(())
        }
        Commands::List => {
            list_packages()?;
            Ok(())
        }
        Commands::Info => {
            get_system_info()?;
            Ok(())
        }
    }
}
