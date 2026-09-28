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

`MIGRATION_DATABASE_URL` utilise le propriétaire de la base uniquement pour appliquer les migrations. `DATABASE_URL` doit impérativement utiliser un rôle non-superutilisateur sans attribut `BYPASSRLS`; le serveur refuse de démarrer autrement. Docker Compose crée le rôle local `iot_app` lors de l'initialisation d'un nouveau volume. Pour une base déjà existante, exécutez `docker/init.sql` avec un administrateur puis relancez les migrations.

Toutes les opérations métier ouvrent une transaction qui définit `app.tenant_id` avec une portée locale. Les politiques PostgreSQL RLS filtrent ensuite les lectures et écritures, même si une requête applicative oublie son prédicat `tenant_id`.

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
| `POST` | `/api/v1/devices/{id}/credentials` | Bearer ADMIN | Remplacer la clé et la révéler une seule fois |
| `DELETE` | `/api/v1/devices/{id}/credentials` | Bearer ADMIN | Révoquer immédiatement la clé active |
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

La rotation incrémente `api_key_version` et rend immédiatement l'ancienne clé inutilisable. La révocation conserve l'appareil et son historique mais bloque toute nouvelle ingestion. Une rotation après révocation produit une nouvelle clé active. Les clés en clair ne sont ni stockées ni récupérables ; conservez la valeur retournée par la création ou la rotation dans un gestionnaire de secrets.

## Qualité

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Le test d'intégration RLS utilise `TEST_DATABASE_URL`. Sans cette variable, il est ignoré afin que les tests unitaires restent exécutables sans PostgreSQL. Avec cette variable, il crée deux tenants temporaires et vérifie qu'une transaction du tenant A ne peut ni voir ni modifier l'appareil du tenant B.

Le document local `gm.md` contient la vision produit et n'est volontairement pas suivi par Git.
