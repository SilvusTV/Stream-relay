use std::path::Path;
use std::process::Command;

// Petite fonction utilitaire pour exécuter une commande système (ex: meson, cmake).
// - On lance la commande
// - Si elle échoue (code de retour ≠ 0), on arrête le build avec un message clair
fn run(cmd: &mut Command) {
    let status = cmd.status().expect("failed to spawn command");
    if !status.success() {
        panic!("Command failed: {:?}", cmd);
    }
}

#[cfg(feature = "rist")]
fn build_and_link_librist() {
    // Dossier racine du sous-module librist (chemin relatif au Cargo.toml du projet)
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("libs").join("librist");
    // On construit dans un dossier "build" à l'intérieur du sous-module
    let build_dir = root.join("build");

    // 1) Configure le projet librist avec Meson en mode Release
    run(
        Command::new("meson")
            .current_dir(&root)
            .arg("setup")
            .arg(&build_dir)
            .arg("--buildtype=release"),
    );
    // 2) Compile librist avec Meson
    run(
        Command::new("meson")
            .current_dir(&root)
            .arg("compile")
            .arg("-C")
            .arg(&build_dir),
    );

    // 3) Indique à Cargo où trouver la bibliothèque compilée
    // librist génère ses .dylib directement dans le dossier build/
    println!("cargo:rustc-link-search=native={}", build_dir.display());
    // On link en dynamique (dylib) la bibliothèque nommée "rist"
    println!("cargo:rustc-link-lib=dylib=rist");
}

#[cfg(feature = "srt")]
fn build_and_link_srt() {
    // Dossier racine du sous-module srt
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("libs").join("srt");
    // Dossier de build dédié
    let build_dir = root.join("build");

    // 1) Configure le projet srt avec CMake (profil Release, sans applications)
    run(
        Command::new("cmake")
            .current_dir(&root)
            .arg("-S")
            .arg(".")
            .arg("-B")
            .arg(&build_dir)
            .arg("-DCMAKE_BUILD_TYPE=Release")
            .arg("-DENABLE_APPS=OFF"),
    );
    // 2) Compile srt avec CMake
    run(
        Command::new("cmake")
            .current_dir(&root)
            .arg("--build")
            .arg(&build_dir)
            .arg("--config")
            .arg("Release"),
    );

    // 3) Indique à Cargo où trouver la bibliothèque compilée
    // srt produit un fichier statique "libsrt.a" directement dans build/
    println!("cargo:rustc-link-search=native={}", build_dir.display());
    // On link en statique la bibliothèque nommée "srt"
    println!("cargo:rustc-link-lib=static=srt");
}

fn main() {
    // Si la feature "rist" est activée, on construit et on link librist
    #[cfg(feature = "rist")]
    build_and_link_librist();

    // Si la feature "srt" est activée, on construit et on link srt
    #[cfg(feature = "srt")]
    build_and_link_srt();
}