# IoT Monitoring Backend

API Rust/Axum multi-tenant pour enregistrer des appareils IoT, ingérer leurs mesures dans TimescaleDB, calculer des séries agrégées et diffuser les mesures et alertes en temps réel.

## Démarrage

Prérequis : Rust stable, Docker et Docker Compose.

```bash
cp .env.example .env
docker compose up -d
cargo run
```

Remplacez impérativement `JWT_SECRET` par une valeur aléatoire d'au moins 32 caractères. Les migrations SQL sont appliquées automatiquement au démarrage.

## API

Toutes les routes métier sont préfixées par `/api/v1`.

| Méthode | Route | Authentification | Usage |
|---|---|---|---|
| `GET` | `/health` | aucune | Liveness |
| `GET` | `/ready` | aucune | Disponibilité de PostgreSQL |
| `POST` | `/api/v1/auth/register` | aucune | Créer un tenant et son administrateur |
| `POST` | `/api/v1/auth/login` | aucune | Obtenir un JWT |
| `GET` | `/api/v1/auth/me` | Bearer | Profil courant |
| `GET/POST` | `/api/v1/devices` | Bearer | Lister/créer les appareils |
| `GET/PUT/DELETE` | `/api/v1/devices/{id}` | Bearer | Consulter/modifier/supprimer un appareil |
| `POST` | `/api/v1/sensor-data/ingest` | Device | Ingérer une mesure |
| `GET` | `/api/v1/sensor-data` | Bearer | Historique filtré et paginé |
| `GET` | `/api/v1/sensor-data/latest` | Bearer | Dernières mesures |
| `GET` | `/api/v1/sensor-data/analytics` | Bearer | Moyenne, minimum et maximum par intervalle |
| `GET/POST` | `/api/v1/alert-rules` | Bearer | Lister/créer les règles |
| `GET` | `/api/v1/alerts` | Bearer | Historique des alertes |
| `PATCH` | `/api/v1/alerts/{id}/status` | Bearer | Acquitter ou résoudre une alerte |
| `GET` | `/api/v1/notifications` | Bearer | Notifications de l'utilisateur |
| `GET` | `/ws/monitoring?token=JWT` | JWT query | Événements WebSocket du tenant |

La création d'un appareil retourne une `api_key` une seule fois. Pour ingérer :

```http
POST /api/v1/sensor-data/ingest
Authorization: Device dev_<uuid>.<secret>
Content-Type: application/json

{
  "event_id": "4b1fb506-91ae-4540-917f-d847a16831b5",
  "metric_type": "temperature",
  "value": 24.7,
  "unit": "°C",
  "timestamp": "2026-09-28T10:00:00Z"
}
```

## Qualité

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Le document local `gm.md` contient la vision produit et n'est volontairement pas suivi par Git.
