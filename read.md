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

---

### Mise à jour 2025-10-30 — Probes SRT/RIST + copie auto des DLL sous Windows

Cette mise à jour ajoute deux éléments importants:

1) Probes SRT/RIST (smoke tests lisibles)
- Fichiers: `src/relay/srt.rs`, `src/relay/rist.rs`, `src/relay/mod.rs`, intégration CLI dans `src/main.rs`.
- SRT: `SrtEndpoint { input, output, latency_ms, mode, state }` avec `new()`, `open()`, `close()`, `describe()`.
  - Détermination du mode: listener si `input` contient `mode=listener` ou `srt://@…`, sinon caller.
  - `open()` simule l’état (LISTENING ou CONNECTED); pas de socket réel ouvert pour l’instant.
  - `describe()` affiche: `[SRT] input=… output=… mode=… state=… latency_ms=…`.
- RIST: `RistEndpoint { input, output, mode, state }` avec `new()`, `open()`, `close()`, `describe()`.
  - Mode: listener si `rist://@…` ou `mode=listener`, sinon caller.
  - `describe()` affiche: `[RIST] input=… output=… mode=… state=…`.
- Runners: `run_srt_probe()` et `run_rist_probe()` ouvrent l’endpoint, impriment l’état, dorment 3s, ferment, réimpriment l’état.
- CLI (clap) — deux sous-commandes:
  - `srt2srt --input <uri> --output <uri> --latency-ms <u64>`
  - `rist2rist --input <uri> --output <uri>`
  Si aucune sous-commande n’est passée, le serveur HTTP Rocket démarre comme avant.

2) Copie automatique des DLL SRT/RIST sous Windows
- Fichier: `build.rs`.
- Lorsque la feature `srt` est activée, le build CMake sort `srt.dll` dans `libs/srt/build/Release`. Après la compilation, le script copie automatiquement `srt.dll` dans `target/{PROFILE}` (`debug` ou `release`).
- Lorsque la feature `rist` est activée, le build Meson sort `librist.dll` (ou parfois `rist.dll`) dans `libs/librist/build`. Le script tente de copier l’un et l’autre vers `target/{PROFILE}`.
- Conséquence: plus besoin de copier les DLLs à la main ou de modifier le `%PATH%` pour exécuter le binaire localement.

#### Comment tester (Windows)

Pré-requis outils si vous activez SRT/RIST:
- MSVC Build Tools (Visual Studio), CMake (pour SRT), Meson+Ninja (pour RIST).

1) API HTTP seule (sans SRT/RIST):
```
cargo run
```
Vérifier:
```
curl http://127.0.0.1:8000/health
curl http://127.0.0.1:8000/stats
curl http://127.0.0.1:8000/metrics
```

2) Build avec SRT et/ou RIST (les DLL seront auto-copiées vers target/…):
```
cargo clean
cargo build --features "srt rist"
```

3) Probes SRT/RIST (smoke tests):
- SRT:
```
cargo run --features "srt rist" -- \
  srt2srt --input "srt://@:9000?mode=listener" \
          --output "srt://127.0.0.1:10000?mode=caller" \
          --latency-ms 80
```
Sortie attendue (exemple):
```
[SRT] input=srt://@:9000?mode=listener output=srt://127.0.0.1:10000?mode=caller mode=listener state=LISTENING latency_ms=80
[SRT] input=srt://@:9000?mode=listener output=srt://127.0.0.1:10000?mode=caller mode=listener state=CLOSED latency_ms=80
```
- RIST:
```
cargo run --features "srt rist" -- \
  rist2rist --input "rist://@:9000?mode=listener" \
            --output "rist://127.0.0.1:10000?mode=caller"
```
Sortie attendue (exemple):
```
[RIST] input=rist://@:9000?mode=listener output=rist://127.0.0.1:10000?mode=caller mode=listener state=LISTENING
[RIST] input=rist://@:9000?mode=listener output=rist://127.0.0.1:10000?mode=caller mode=listener state=CLOSED
```

4) Lancer juste le serveur HTTP (avec features actives, mais sans action CLI):
```
cargo run --features "srt rist"
```

Notes:
- Les probes n’ouvrent pas encore de sockets réels; elles servent uniquement de test visuel (URIs, mode, état). La logique de transport sera branchée plus tard.
- Sous Linux/macOS, la copie de DLL ne s’applique pas. `build.rs` configure le lien en conséquence (static pour SRT côté non-Windows, et `dylib=rist` standard pour RIST) et il n’y a pas besoin de copier des `.so`/`.dylib` pour ces smoke tests actuels.

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

---

### Mise à jour 2025-10-30 — Probes SRT/RIST + copie auto des DLL sous Windows

Cette mise à jour ajoute deux éléments importants:

1) Probes SRT/RIST (smoke tests lisibles)
- Fichiers: `src/relay/srt.rs`, `src/relay/rist.rs`, `src/relay/mod.rs`, intégration CLI dans `src/main.rs`.
- SRT: `SrtEndpoint { input, output, latency_ms, mode, state }` avec `new()`, `open()`, `close()`, `describe()`.
  - Détermination du mode: listener si `input` contient `mode=listener` ou `srt://@…`, sinon caller.
  - `open()` simule l’état (LISTENING ou CONNECTED); pas de socket réel ouvert pour l’instant.
  - `describe()` affiche: `[SRT] input=… output=… mode=… state=… latency_ms=…`.
- RIST: `RistEndpoint { input, output, mode, state }` avec `new()`, `open()`, `close()`, `describe()`.
  - Mode: listener si `rist://@…` ou `mode=listener`, sinon caller.
  - `describe()` affiche: `[RIST] input=… output=… mode=… state=…`.
- Runners: `run_srt_probe()` et `run_rist_probe()` ouvrent l’endpoint, impriment l’état, dorment 3s, ferment, réimpriment l’état.
- CLI (clap) — deux sous-commandes:
  - `srt2srt --input <uri> --output <uri> --latency-ms <u64>`
  - `rist2rist --input <uri> --output <uri>`
  Si aucune sous-commande n’est passée, le serveur HTTP Rocket démarre comme avant.

2) Copie automatique des DLL SRT/RIST sous Windows
- Fichier: `build.rs`.
- Lorsque la feature `srt` est activée, le build CMake sort `srt.dll` dans `libs/srt/build/Release`. Après la compilation, le script copie automatiquement `srt.dll` dans `target/{PROFILE}` (`debug` ou `release`).
- Lorsque la feature `rist` est activée, le build Meson sort `librist.dll` (ou parfois `rist.dll`) dans `libs/librist/build`. Le script tente de copier l’un et l’autre vers `target/{PROFILE}`.
- Conséquence: plus besoin de copier les DLLs à la main ou de modifier le `%PATH%` pour exécuter le binaire localement.

#### Comment tester (Windows)

Pré-requis outils si vous activez SRT/RIST:
- MSVC Build Tools (Visual Studio), CMake (pour SRT), Meson+Ninja (pour RIST).

1) API HTTP seule (sans SRT/RIST):
```
cargo run
```
Vérifier:
```
curl http://127.0.0.1:8000/health
curl http://127.0.0.1:8000/stats
curl http://127.0.0.1:8000/metrics
```

2) Build avec SRT et/ou RIST (les DLL seront auto-copiées vers target/…):
```
cargo clean
cargo build --features "srt rist"
```

3) Probes SRT/RIST (smoke tests):
- SRT:
```
cargo run --features "srt rist" -- \
  srt2srt --input "srt://@:9000?mode=listener" \
          --output "srt://127.0.0.1:10000?mode=caller" \
          --latency-ms 80
```
Sortie attendue (exemple):
```
[SRT] input=srt://@:9000?mode=listener output=srt://127.0.0.1:10000?mode=caller mode=listener state=LISTENING latency_ms=80
[SRT] input=srt://@:9000?mode=listener output=srt://127.0.0.1:10000?mode=caller mode=listener state=CLOSED latency_ms=80
```
- RIST:
```
cargo run --features "srt rist" -- \
  rist2rist --input "rist://@:9000?mode=listener" \
            --output "rist://127.0.0.1:10000?mode=caller"
```
Sortie attendue (exemple):
```
[RIST] input=rist://@:9000?mode=listener output=rist://127.0.0.1:10000?mode=caller mode=listener state=LISTENING
[RIST] input=rist://@:9000?mode=listener output=rist://127.0.0.1:10000?mode=caller mode=listener state=CLOSED
```

4) Lancer juste le serveur HTTP (avec features actives, mais sans action CLI):
```
cargo run --features "srt rist"
```

Notes:
- Les probes n’ouvrent pas encore de sockets réels; elles servent uniquement de test visuel (URIs, mode, état). La logique de transport sera branchée plus tard.
- Sous Linux/macOS, la copie de DLL ne s’applique pas. `build.rs` configure le lien en conséquence (static pour SRT côté non-Windows, et `dylib=rist` standard pour RIST) et il n’y a pas besoin de copier des `.so`/`.dylib` pour ces smoke tests actuels.

---

### Mise à jour 2025-10-30 (bis) — Auto‑run implicite des probes selon les features

Nouveau comportement demandé: « plus d’implicite ».

- Quand vous lancez `cargo run --features srt`, le binaire démarre le serveur HTTP, puis lance automatiquement un probe SRT en arrière‑plan avec des valeurs par défaut. Idem pour `--features rist`.
- Si les deux features sont activées (`--features "srt rist"`), les deux probes démarrent automatiquement.
- Tout est loggé dans le terminal: adresse HTTP, URLs utiles, et les lignes de status des probes `[SRT]` / `[RIST]` avec les URIs exactes.

Implémentation technique:
- Ajout d’un fairing Rocket `AdHoc::on_liftoff("auto-probes", ...)` dans `build_rocket()` qui démarre les tâches asynchrones après que le serveur HTTP ait pris son port.
- Tâches de fond: `relay::start_srt_auto(...)` et `relay::start_rist_auto(...)` qui:
  - ouvrent l’endpoint (simulé),
  - loggent `describe()` immédiatement,
  - ré‑impriment périodiquement l’état (toutes les 60s) pour garder les URLs sous la main,
  - bouclent avec une petite pause en cas d’erreur.

Valeurs par défaut (surchargées par ENV):
- SRT
  - `SRTRIST_SRT_INPUT` (def: `srt://@:9000?mode=listener`)
  - `SRTRIST_SRT_OUTPUT` (def: `srt://127.0.0.1:10000?mode=caller`)
  - `SRTRIST_SRT_LATENCY_MS` (def: `80`)
  - Désactiver: `SRTRIST_AUTO_SRT=0`
- RIST
  - `SRTRIST_RIST_INPUT` (def: `rist://@:10000?mode=listener`)
  - `SRTRIST_RIST_OUTPUT` (def: `rist://127.0.0.1:11000?mode=caller`)
  - Désactiver: `SRTRIST_AUTO_RIST=0`

Bannières au démarrage:
- `[INIT] HTTP server listening on <addr>:<port>`
- `[INFO] URLs: http://<addr>:<port>/health  http://<addr>:<port>/stats  http://<addr>:<port>/metrics`
- `[INIT] Auto SRT probe enabled` et/ou `[INIT] Auto RIST probe enabled` + rappel des URIs défaut.

Comment tester l’implicite:
- SRT seul:
```powershell
cargo run --features srt
```
Attendu en console (exemple):
```
[INIT] HTTP server listening on 127.0.0.1:8000
[INFO] URLs: http://127.0.0.1:8000/health  http://127.0.0.1:8000/stats  http://127.0.0.1:8000/metrics
[INIT] Auto SRT probe enabled
[INIT] SRT defaults: input=srt://@:9000?mode=listener output=srt://127.0.0.1:10000?mode=caller latency_ms=80
[SRT] input=srt://@:9000?mode=listener output=srt://127.0.0.1:10000?mode=caller mode=listener state=LISTENING latency_ms=80
```
- RIST seul:
```powershell
cargo run --features rist
```
- Les deux:
```powershell
cargo run --features "srt rist"
```
- Désactiver un auto‑probe au besoin (exemples PowerShell):
```powershell
$env:SRTRIST_AUTO_SRT = "0"; cargo run --features "srt rist"
$env:SRTRIST_SRT_INPUT = "srt://@:12000?mode=listener"; cargo run --features srt
```

Remarques:
- Les sous‑commandes explicites (`srt2srt`, `rist2rist`) continuent de fonctionner; si vous les utilisez, le programme exécute la probe demandée puis termine (comportement inchangé).
- Windows: la copie automatique des DLL dans `target/{debug,release}` reste en place; aucune action manuelle n’est nécessaire pour lancer le binaire après `cargo build`.


---

### Milestone 2 — Transports réels (SRT & RIST) branchés à la pipe — 2025-10-30

#### Ce qui a été mis en place
- Fondation « transport » commune:
  - `src/structures/error.rs` définit `TransportError` (via `thiserror`) et l’alias `TResult<T>`.
  - `src/relay/transport.rs` expose 3 traits asynchrones minimalistes:
    - `TransportRx::recv(&mut self, buf) -> TResult<usize>`
    - `TransportTx::send(&mut self, buf) -> TResult<usize>`
    - `TransportMeta::{open, close, describe}`
- Pipe réutilisable:
  - `src/relay/pipe.rs` fournit `run_pipe<Rx, Tx>(rx, tx)`:
    - Tampon réutilisé de 64 KiB
    - Boucle `recv → send`
    - Sur `TransportError::Timeout`: courte attente (5 ms) puis reprise; sur toute autre erreur: log et sortie propre
- SRT — première implémentation fonctionnelle:
  - `src/relay/srt.rs` implémente `SrtReceiver` (RX) et `SrtSender` (TX)
  - Parsing d’URI simple `srt://`:
    - RX: `srt://@:PORT` ou `...?mode=listener` pour écouter
    - TX: `srt://HOST:PORT` pour émettre
  - I/O réelles avec `tokio::net::UdpSocket` (V1 de travail): on transfère réellement des octets en local, avec timeouts mappés en `TransportError::Timeout`
  - Implémente `TransportRx`, `TransportTx`, `TransportMeta`
- RIST — première implémentation fonctionnelle:
  - `src/relay/rist.rs` implémente `RistReceiver`/`RistSender` avec la même logique et les URIs `rist://`
  - I/O via UDP comme pour SRT (V1), timeouts gérés
- Câblage du relais (wiring):
  - `src/relay/mod.rs` expose la pipe et des helpers:
    - `run_srt_probe` / `run_rist_probe` créent les endpoints et appellent `run_pipe`
    - `start_srt_auto` / `start_rist_auto` lancent la pipe en tâche de fond
  - Les prints de démarrage affichent systématiquement les URIs d’entrée/sortie
- Dépendances Cargo:
  - Ajout de `thiserror`, `async-trait`, et activation des features tokio nécessaires (`time`, `net`, `rt-multi-thread`, `macros`)

> Note: cette V1 utilise UDP sous le capot pour valider la pipe et le câblage. Le `build.rs` reste prêt pour les features `srt`/`rist` (FFI C), mais le runtime actuel ne dépend pas encore des bibliothèques C.

#### Comment tester que ça fonctionne
Vous avez deux modes: « probe CLI » (process éphémère) et « auto-probes » (lancées avec le serveur HTTP).

1) Test SRT en boucle locale (probe CLI)
- Terminal A (lance la pipe: écoute sur 9000 → envoie vers 10000):
```powershell
cargo run -- srt2srt --input srt://@:9000?mode=listener --output srt://127.0.0.1:10000?mode=caller --latency-ms 80
```
- Terminal B (envoyer des données vers l’entrée sur 9000). Quelques options possibles:
  - Avec ffmpeg (si dispo):
    ```powershell
    ffmpeg -re -f lavfi -i testsrc=size=320x240:rate=25 -f mpegts udp://127.0.0.1:9000
    ```
  - Avec PowerShell (envoi de bytes arbitraires):
    ```powershell
    $udpClient = New-Object System.Net.Sockets.UdpClient
    $bytes = [System.Text.Encoding]::UTF8.GetBytes("hello-srt")
    $udpClient.Connect('127.0.0.1',9000)
    [void]$udpClient.Send($bytes,$bytes.Length)
    $udpClient.Close()
    ```
- Observation attendue: le binaire affiche les URIs `input`/`output`; la pipe transfère les paquets reçus sur 9000 vers 127.0.0.1:10000.
  - Pour voir la sortie côté 10000, vous pouvez écouter avec un autre outil (ex: Wireshark, `socat - UDP4-RECVFROM:10000` sous WSL, ou un petit script UDP).

2) Test RIST en boucle locale (probe CLI)
- Terminal A:
```powershell
cargo run -- rist2rist --input rist://@:10000?mode=listener --output rist://127.0.0.1:11000?mode=caller
```
- Terminal B: envoyez des datagrammes sur `127.0.0.1:10000` comme ci‑dessus. Les octets doivent être relayés vers `127.0.0.1:11000`.

3) Test via serveur HTTP et auto-probes
- Lancer l’appli sans sous‑commande:
```powershell
cargo run
```
- Par défaut, l’appli démarre des auto-probes si les features sont activées à la compilation:
  - `#[cfg(feature = "srt")]` pour SRT
  - `#[cfg(feature = "rist")]` pour RIST
- Variables d’environnement disponibles pour surcharger:
  - SRT:
    - `SRTRIST_AUTO_SRT=0` pour désactiver l’auto-probe
    - `SRTRIST_SRT_INPUT` (par défaut `srt://@:9000?mode=listener`)
    - `SRTRIST_SRT_OUTPUT` (par défaut `srt://127.0.0.1:10000?mode=caller`)
    - `SRTRIST_SRT_LATENCY_MS` (par défaut `80`)
  - RIST:
    - `SRTRIST_AUTO_RIST=0` pour désactiver
    - `SRTRIST_RIST_INPUT` (défaut `rist://@:10000?mode=listener`)
    - `SRTRIST_RIST_OUTPUT` (défaut `rist://127.0.0.1:11000?mode=caller`)
- Vous devriez voir au démarrage: l’adresse HTTP et les URIs d’entrée/sortie pour chaque auto-probe active.
- Endpoints HTTP:
  - `http://127.0.0.1:8000/health`
  - `http://127.0.0.1:8000/stats`
  - `http://127.0.0.1:8000/metrics`

4) Remarques Windows (features FFI activées)
- Si vous compilez avec `--features srt` et/ou `--features rist`, le `build.rs` construit les libs C et lie en dynamique sous Windows.
- Il faut que `srt.dll` (et la DLL RIST) soient trouvées au runtime. Deux solutions:
  - Copier les DLLs à côté du binaire (`target/debug` ou `target/release`)
  - Ou ajouter les dossiers `libs/srt/build/Release` et `libs/librist/build` au `%PATH%` de la session avant d’exécuter

#### Ce que ça valide aujourd’hui
- L’API commune de transport fonctionne: on peut brancher n’importe quel `Receiver`/`Sender` conforme aux traits et les relier avec la même `run_pipe`.
- Les URIs simplifiées sont suffisantes pour des tests locaux et montrent le « débit réel » entre deux sockets.
- La boucle gère un timeout non-bloquant et effectue un backoff minimal.

#### Ce qu’il manque pour être pleinement fonctionnel (prochaines étapes)
- SRT/RIST réels (FFI):
  - Remplacer l’implémentation UDP par des wrappers sûrs autour de `libsrt` et `librist` (via `bindgen` ou crates dédiées)
  - Gérer la configuration SRT (latence, payload size, modes caller/listener complets, encryption si activée)
  - Gérer les profils RIST (simple/advanced), clés, retransmissions, NAKs, etc.
- Robustesse réseau:
  - Reconnexions automatiques, backoff exponentiel, journalisation des transitions d’état
  - Gestion propre de la fermeture (`close`) et des erreurs transitoires vs fatales
- Observabilité et métriques réseau:
  - Débits, pertes, latences/jitter, compteurs de reconnexions, erreurs par code/protocole
  - Exposer ces métriques sur `/metrics` et dans `/stats`
- Parsing d’URI plus complet:
  - Support des paramètres SRT/RIST standards (`latency`, `pbkeylen`, `passphrase`, `mode`, `streamid`, etc.)
  - Validation stricte et messages d’erreur détaillés
- CLI et configuration:
  - Sous-commandes plus riches (`srt listen/connect`, `rist listen/connect`, fichiers de config, ENV)
  - Logs structurés (`tracing`) et niveaux configurables
- Packaging/CI:
  - Script de copie automatique des DLLs sur Windows
  - Tests unitaires/integration (incl. pipe sur sockets), et CI

#### TL;DR
- Vous pouvez déjà vérifier un transfert réel d’octets en local via les sous-commandes `srt2srt` et `rist2rist`.
- Le cœur (traits + pipe) est en place et réutilisable.
- La prochaine grosse marche consiste à brancher les vraies bibliothèques SRT/RIST à la place des sockets UDP et à instrumenter les métriques.
