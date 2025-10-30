use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::env;

// Petite fonction utilitaire pour exécuter une commande système (ex: meson, cmake).
// - On lance la commande
// - Si elle échoue (code de retour ≠ 0), on arrête le build avec un message clair
fn run(cmd: &mut Command) {
    let status = cmd.status().expect("failed to spawn command");
    if !status.success() {
        panic!("Command failed: {:?}", cmd);
    }
}

#[cfg(target_os = "windows")]
fn target_profile_dir() -> PathBuf {
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target").join(profile)
}

#[cfg(target_os = "windows")]
fn copy_dll_if_exists(src: &Path, dst_dir: &Path) {
    if src.exists() {
        if let Err(e) = fs::create_dir_all(dst_dir) {
            println!("cargo:warning=failed to create target dir {}: {}", dst_dir.display(), e);
            return;
        }
        let dst = dst_dir.join(src.file_name().unwrap());
        match fs::copy(src, &dst) {
            Ok(_) => println!("cargo:warning=Copied {} -> {}", src.display(), dst.display()),
            Err(e) => println!("cargo:warning=Failed to copy {} -> {}: {}", src.display(), dst.display(), e),
        }
    } else {
        println!("cargo:warning=DLL not found (optional copy skipped): {}", src.display());
    }
}

#[cfg(feature = "rist")]
fn build_and_link_librist() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_else(|_| String::new());
    // Dossier racine du sous-module librist (chemin relatif au Cargo.toml du projet)
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("libs")
        .join("librist");
    // On construit dans un dossier "build" à l'intérieur du sous-module
    let build_dir = root.join("build");

    if target_os == "windows" {
        run(Command::new("meson")
            .current_dir(&root)
            .arg("setup")
            .arg(&build_dir)
            .arg("--buildtype=release")
            .arg("-Dbuilt_tools=false")
            .arg("-Dtest=false")
            .arg("--wipe"));
    } else {
        let mut cmd = Command::new("meson");
        cmd
            .current_dir(&root)
            .arg("setup")
            .arg(&build_dir)
            .arg("--buildtype=release");
        // macOS: forcer la lib statique pour éviter la dépendance runtime .dylib
        if target_os == "macos" {
            cmd.arg("--default-library=static");
        }
        run(&mut cmd);
    }
    // 2) Compile librist avec Meson
    run(Command::new("meson")
        .current_dir(&root)
        .arg("compile")
        .arg("-C")
        .arg(&build_dir));

    // 3) Indique à Cargo où trouver la bibliothèque compilée
    // librist génère ses artefacts directement dans le dossier build/
    println!("cargo:rustc-link-search=native={}", build_dir.display());
    // macOS: lier statiquement; autres OS non-Windows: lier dynamiquement par défaut
    if target_os == "windows" {
        println!("cargo:rustc-link-lib=dylib=librist");
        // Copie automatique de la DLL dans target/{profile}
        #[cfg(target_os = "windows")]
        {
            let dst_dir = target_profile_dir();
            let dll1 = build_dir.join("librist.dll");
            let dll2 = build_dir.join("rist.dll");
            // Tenter les deux noms possibles selon la config Meson
            copy_dll_if_exists(&dll1, &dst_dir);
            copy_dll_if_exists(&dll2, &dst_dir);
        }
    } else if target_os == "macos" {
        println!("cargo:rustc-link-lib=static=rist");
    } else {
        println!("cargo:rustc-link-lib=dylib=rist");
    }
}

#[cfg(feature = "srt")]
fn build_and_link_srt() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_else(|_| String::new());
    // Dossier racine du sous-module srt
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("libs")
        .join("srt");
    // Dossier de build dédié
    let build_dir = root.join("build");

    // 1) Configure le projet srt avec CMake (profil Release, sans applications)
    {
        let mut cmd = Command::new("cmake");
        cmd
            .current_dir(&root)
            .arg("-S")
            .arg(".")
            .arg("-B")
            .arg(&build_dir)
            .arg("-DCMAKE_BUILD_TYPE=Release")
            .arg("-DENABLE_APPS=OFF");
        // macOS uniquement: compiler SRT en statique pour éviter la dépendance runtime .dylib
        if target_os == "macos" {
            cmd.arg("-DENABLE_SHARED=OFF").arg("-DENABLE_STATIC=ON");
        }
        run(&mut cmd);
    }
    // 2) Compile srt avec CMake
    run(Command::new("cmake")
        .current_dir(&root)
        .arg("--build")
        .arg(&build_dir)
        .arg("--config")
        .arg("Release"));

    // 3) Indique à Cargo où trouver la bibliothèque compilée
    if target_os == "windows" {
        // Sous Windows avec un build multi-config, CMake place les artefacts dans build/Release
        let lib_dir = build_dir.join("Release");
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        // On link en dynamique la bibliothèque nommée "srt" (srt.lib -> srt.dll)
        println!("cargo:rustc-link-lib=dylib=srt");
        // Copie automatique de srt.dll vers target/{profile}
        #[cfg(target_os = "windows")]
        {
            let dst_dir = target_profile_dir();
            let dll = lib_dir.join("srt.dll");
            copy_dll_if_exists(&dll, &dst_dir);
        }
    } else {
        println!("cargo:rustc-link-search=native={}", build_dir.display());
        if target_os == "macos" {
            // macOS: lier statiquement
            println!("cargo:rustc-link-lib=static=srt");
        } else {
            // autres OS non-Windows: lier dynamiquement
            println!("cargo:rustc-link-lib=dylib=srt");
        }
    }
}

fn main() {
    // Si la feature "rist" est activée, on construit et on link librist
    #[cfg(feature = "rist")]
    build_and_link_librist();

    // Si la feature "srt" est activée, on construit et on link srt
    #[cfg(feature = "srt")]
    build_and_link_srt();
}
