# Cahier des Charges — Projet `stream-relay` (Rust)

## 🧭 Introduction

Le projet **stream-relay** consiste à développer un outil en **Rust** capable de relayer des flux **RIST** et **SRT** en temps réel, tout en offrant un haut niveau de fiabilité, de performance et d’observabilité.

Concrètement, l’application permettra de recevoir un flux vidéo/audio entrant, de le retransmettre vers une autre destination réseau, et de suivre en continu l’état du transfert (débit, latence, pertes, reconnexions, etc.).  
L’objectif est de proposer une alternative moderne, performante et maintenable aux outils existants tels que `rist2rist` ou `srt-live-transmit`, en intégrant dans un seul binaire :
- une **interface unifiée** pour les deux protocoles ;
- un **système de logs JSON** lisible par des outils comme Grafana ou Loki ;
- un **serveur HTTP** exposant les métriques Prometheus ;
- et une **gestion automatique des erreurs et reconnexions**.

Ce projet s’inscrit dans une logique d’**outillage professionnel** pour les environnements de production vidéo (OBS, Moblin, Larix, etc.), et servira de base à de futurs développements autour du transport vidéo fiable en Rust.

## 1️⃣ Objectif

Créer un **binaire unique** `stream-relay` capable de relayer des flux :
- **RIST → RIST**
- **SRT → SRT**

Les deux sous-commandes partagent :
- la même **infrastructure de logs JSON** (`tracing`)  
- un **serveur HTTP** exposant `/health` et `/metrics` (format Prometheus)  

L’objectif : offrir un **relais faible latence, stable et observable**, utilisable entre des outils comme **Larix**, **Moblin**, **OBS** ou des infrastructures de streaming IRL.

---

## 2️⃣ Cas d’usage

- Relier un flux **Larix/Moblin** à un **OBS** via RIST ou SRT.  
- Servir de **“dernier kilomètre”** dans une chaîne de diffusion.  
- Fournir un outil fiable à intégrer dans une stack existante (Node.js, Laravel, Adonis).  
- Surveiller en temps réel les performances et les métriques réseau.

---

## 3️⃣ Rôles & acteurs

- **Opérateur** : configure et lance le relais.  
- **Systèmes externes** : encodeurs (Larix, Moblin), décodeurs (OBS), systèmes de monitoring (Prometheus, Grafana).  

---

## 4️⃣ Fonctions principales

1. **Relais protocolaire** :  
   - Entrée et sortie via URI (`rist://`, `srt://`).  
   - Gestion des reconnexions automatiques.  
2. **Observabilité** :  
   - Endpoints `/health` et `/metrics`.  
   - Exposition Prometheus des statistiques principales :  
     - `relay_peers`, `relay_bitrate_kbps`, `relay_reconnects_total`, `relay_packets_lost_total`, `relay_jitter_ms`.  
3. **Logs JSON** :  
   - Format structuré, compatible Grafana/Loki.  
   - Niveaux configurables (`info`, `debug`, `trace`).  
4. **Configuration** :  
   - CLI et variables d’environnement (`RUST_LOG`, `HTTP_ADDR`, etc.).  
5. **Sécurité minimale** :  
   - Pas de secrets en clair dans les logs.  
   - Codes de sortie cohérents.

---

## 5️⃣ Fonctions avancées (v2)

- Multi-sorties (fan-out).  
- Reconfiguration à chaud.  
- Export WebSocket pour monitoring.  
- Buffer temporel paramétrable.

---

## 6️⃣ Contraintes non-fonctionnelles

- **Stabilité** : pas de fuite mémoire ni de crash silencieux.  
- **Performance** : latence ajoutée < 10 ms.  
- **Portabilité** : Linux x86_64 (priorité), ARM possible.  
- **Containerisation** : Dockerfile multi-stage.  
- **Licences** : conformité BSD-2 (librist) et MPL-2.0 (libsrt).

---

## 7️⃣ Critères d’acceptation

- `GET /health` renvoie `200` en moins de 100 ms.  
- `/metrics` expose les 5 métriques principales.  
- Le relais se reconnecte automatiquement en cas de perte d’entrée.  
- Logs parsables (JSON) et sans secret.  
- Démarrage du binaire en moins de 200 ms.  

---

## 8️⃣ Architecture logicielle

- **Main** : gestion CLI, initialisation des logs et HTTP.  
- **Modules** :  
  - `rist` et `srt` implémentent un trait commun `TransportRelay`.  
  - `common/` gère logs, config, métriques.  
- **Boucles de relais** :  
  - `Reader` → lecture flux.  
  - `Writer` → écriture flux.  
  - `Pipe` → transfert, mesures, reconnexions.

---

## 9️⃣ Rust & FFI (interop C)

- Intégration de **librist** et **libsrt** via FFI :  
  - Génération des bindings automatiques (`bindgen`).  
  - Chargement dynamique (`libloading`) si besoin.  
  - Gestion mémoire sécurisée (`Drop`, `Result`, `CString`).  
- Bonnes pratiques :  
  - Isoler tout le code `unsafe`.  
  - Convertir les codes d’erreurs C → `Result<T, Error>`.  
  - Protéger les callbacks et pointeurs.  

---

## 🔟 Livrables & validation finale

- **Code source Rust** documenté et compilable (`cargo build --release`).  
- **Dockerfile** fonctionnel, image < 100 Mo.  
- **Documentation** : README clair avec exemples CLI.  
- **Tests** :  
  - Unitaires (URI, backoff, parsing).  
  - Intégration (boucles pipe).  
  - E2E (flux SRT/ RIST).  

---

> ✨ **Résumé** : un projet Rust robuste, modulaire et interopérable, servant de pont temps réel pour flux RIST/SRT, avec une observabilité complète et une structure professionnelle prête à évoluer.
