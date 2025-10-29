### Refactor structure du projet — HTTP, modules et métriques (2025-10-29)

#### Pourquoi cette refactor ?
L’implémentation précédente utilisait `include!()` pour injecter des fichiers dans d’autres. C’est non‑idiomatique en Rust et source d’erreurs (portée des imports, visibilité, duplication). De plus, la fonction `#[launch]` de Rocket se trouvait dans `src/web_server.rs` alors que `main.rs` était vide, rendant l’entrée du programme peu claire.

Objectifs de la refactor:
- Remplacer `include!()` par de « vrais » modules Rust.
- Centraliser le point d’entrée dans `src/main.rs` via `#[rocket::main]` et `async fn main()`.
- Structurer les types (réponses JSON et métriques) dans un module `structures` propre.
- Ajouter un endpoint `/metrics` Prometheus et un fairing de métriques HTTP clair.

#### Nouvelle arborescence (parties HTTP)
- `src/main.rs` → point d’entrée via `#[rocket::main]` (async `main`) + `build_rocket()`. 
- `src/web/mod.rs` → fairing `HttpMetricsFairing` et namespace `web`.
- `src/web/routes.rs` → endpoints HTTP: `/health`, `/stats`, `/metrics`.
- `src/structures/mod.rs` → ré‑exports des types publics.
- `src/structures/health.rs` → `HealthResponse` (pub).
- `src/structures/stats_data.rs` → `StatsData`, `StatsResponse` (pub).
- `src/structures/metrics.rs` → `Metrics` + enregistrement Prometheus + export texte.

Note: `src/web_server.rs` a été supprimé pour éviter toute confusion. L’unique point d’entrée est désormais `src/main.rs` via `#[rocket::main]` qui lance l’instance construite par `build_rocket()`.

#### Changements de code majeurs
- Remplacement de `include!(...)` par des imports de module idiomatiques.
- `HealthResponse.code` passe de `&'static i32` à `u16` (type HTTP plus propre), et les structs deviennent `pub` pour être exposés proprement.
- Ajout du module `structures::metrics` avec `Metrics::new()` qui:
  - enregistre `http_requests_total`, `http_request_duration_seconds` et `uptime_seconds` dans un `Registry` Prometheus;
  - fournit `gather_text()` pour l’endpoint `/metrics`.
- Ajout du fairing `web::HttpMetricsFairing` qui mesure la latence par requête et incrémente les compteurs par méthode/statut.
- `main.rs` possède maintenant une `async fn main()` (via `#[rocket::main]`) qui:
  - construit l'instance Rocket via `build_rocket()`;
  - lance le serveur avec `launch().await`;
  - peut effectuer d’autres initialisations avant le lancement (logs, lecture d’ENV, tâches en arrière‑plan) ;
  - monte les routes `health`, `stats_endpoint`, `metrics_export` et attache le fairing.

#### Endpoints disponibles après refactor
- `GET /health` → `{ "status": "ok", "code": 200 }`
- `GET /stats` → structure inspirée de `TemplateStatsResponse.json` avec des valeurs par défaut (à brancher sur les vraies métriques du relais par la suite).
- `GET /metrics` → export Prometheus (texte)

#### Lancer l’application (dev)
```powershell
cargo run
```
Puis interroger:
```powershell
curl http://127.0.0.1:8000/health
curl http://127.0.0.1:8000/stats
curl http://127.0.0.1:8000/metrics
```

> Remarque: Si vous activez des features FFI (ex: `--features srt`), reportez‑vous aux notes de liaison plus bas.

---

### Rapport de build — mise à jour du 2025-10-29 10:17

#### Ce que j’ai refait
- Commande exécutée: `cargo build --features "srt"`.
- L’erreur OpenSSL précédente est bien résolue (CMake a pu configurer/compilier SRT).
- Le build échoue désormais à l’étape de l’éditeur de liens (link.exe) côté binaire Rust.

#### Erreur observée
```
LINK : fatal error LNK1181: cannot open input file 'srt.lib'
```

#### Constats (ce que produit CMake)
Dans `libs\srt\build\Release` j’obtiens:
```
srt.dll
srt.lib            (import lib pour la DLL)
srt_static.lib     (lib statique)
```
Mais le script `build.rs` indique actuellement à Cargo/Rustc de chercher les libs dans `libs\srt\build` (sans le sous-dossier `Release`) et demande un lien `static=srt`.

Conséquences:
- Le lien échoue car `srt.lib` et `srt_static.lib` se trouvent en réalité dans `libs\srt\build\Release`.
- De plus, pour du statique, le nom attendu serait `srt_static.lib` (et non `srt.lib`). `srt.lib` sert d’import lib pour la DLL `srt.dll` (lien dynamique).

#### Cause racine
- Mismatch de chemin (on pointe sur `build/` au lieu de `build/Release/`).
- Mismatch de nom/type de lib (on demande `static=srt` alors que la lib statique se nomme `srt_static.lib`).

---

### Comment corriger
Vous avez deux voies simples. Je recommande la A (lien dynamique) pour démarrer rapidement.

#### A) Lien dynamique (recommandé)
Adapter `build.rs` (partie SRT) pour:
- Chercher dans le bon répertoire
- Lier la DLL (`dylib=srt`)

Exemple de modifications conceptuelles dans `build.rs`:
```
// après la compilation CMake
// Exemple: pointer le dossier Release et lier la DLL SRT
println!("cargo:rustc-link-search=native=<chemin\\vers\\libs\\srt\\build\\Release>");
println!("cargo:rustc-link-lib=dylib=srt");
```
Remarques:
- À l’exécution, Windows doit trouver `srt.dll`. Deux options:
  - Copier la DLL à côté du binaire de sortie:
    ```powershell
    Copy-Item .\libs\srt\build\Release\srt.dll .\target\debug\
    ```
  - Ou ajouter le dossier au `PATH` avant de lancer votre binaire:
    ```powershell
    $env:PATH = "$(Get-Location)\libs\srt\build\Release;" + $env:PATH
    ```
- Rebuild ensuite:
  ```powershell
  cargo clean
  cargo build --features "srt"
  ```

#### B) Lien statique (sans DLL)
Si vous préférez lier statiquement SRT:
- Chercher dans `build/Release`
- Lier `static=srt_static`
- Ajouter les libs système Windows requises, et très probablement les libs OpenSSL si SRT a été construit contre OpenSSL (fréquent par défaut).

Exemple de directives à émettre depuis `build.rs`:
```rust
println!("cargo:rustc-link-search=native={}", release_dir.display());
println!("cargo:rustc-link-lib=static=srt_static");

// Libs système Windows courantes pour SRT
println!("cargo:rustc-link-lib=ws2_32");
println!("cargo:rustc-link-lib=iphlpapi");
println!("cargo:rustc-link-lib=bcrypt");
println!("cargo:rustc-link-lib=crypt32");

// Si SRT a été construit avec OpenSSL (cas par défaut):
// Indiquez aussi l’emplacement des libs OpenSSL et liez-les
// (adaptez le chemin selon votre installation)
println!("cargo:rustc-link-search=native=C:\\Program Files\\OpenSSL-Win64\\lib");
println!("cargo:rustc-link-lib=ssl");
println!("cargo:rustc-link-lib=crypto");
```
Notes:
- Selon votre installation OpenSSL (Shining Light ou vcpkg), les noms/chemins des `.lib` peuvent varier. Avec Shining Light, c’est souvent `libssl.lib` et `libcrypto.lib` dans `...\OpenSSL-Win64\lib`.
- Si vous voyez ensuite des `unresolved external symbol` liés à OpenSSL, c’est qu’il manque encore la recherche (`rustc-link-search`) ou le bon nom de lib.

#### C) Alternative: modifier la sortie CMake
Vous pouvez aussi éviter le sous-dossier `Release` en forçant CMake à déposer ses artefacts directement sous `build/`:
- Ajouter lors de la configuration CMake des variables comme:
  - `-DCMAKE_RUNTIME_OUTPUT_DIRECTORY="<chemin>"`
  - `-DCMAKE_ARCHIVE_OUTPUT_DIRECTORY="<chemin>"`
  - `-DCMAKE_LIBRARY_OUTPUT_DIRECTORY="<chemin>"`
Cela évite d’ajuster `build.rs`, mais requiert de modifier la commande CMake dans `build.rs`.

---

### Résumé
- OpenSSL: OK désormais.
- Nouvelle erreur: le linker ne trouve pas `srt.lib` car `build.rs` pointe sur le mauvais dossier et demande le mauvais type de lib.
- Solutions:
  - A) Lien dynamique: pointer `build/Release`, lier `dylib=srt`, veiller à la présence de `srt.dll` à l’exécution.
  - B) Lien statique: pointer `build/Release`, lier `static=srt_static`, ajouter `ws2_32`, `iphlpapi`, `bcrypt`, `crypt32` et les libs OpenSSL.
  - C) Ou forcer CMake à sortir les artefacts directement sous `build/`.

Dites-moi quelle option vous préférez; je peux appliquer la modification correspondante dans `build.rs` et recompiler pour valider.