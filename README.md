# abm

> Breve descripción del proyecto. _(TODO: completar)_

Plataforma de datos de series temporales basada en **TimescaleDB** (PostgreSQL 16)
y **Grafana**, orquestada con Docker Compose.

## Requisitos

- [Docker](https://docs.docker.com/get-docker/) y Docker Compose v2

## Puesta en marcha

```bash
cd infra
cp .env.example .env      # edita las credenciales
docker compose up -d
```

## Servicios

| Servicio    | Imagen                             | URL / Puerto            |
|-------------|------------------------------------|-------------------------|
| TimescaleDB | `timescale/timescaledb:latest-pg16`| `localhost:5432`        |
| Grafana     | `grafana/grafana:latest`           | http://localhost:3000   |

La primera vez, Grafana pide usuario/contraseña (`admin` / `admin`) y obliga a cambiarla.

## Variables de entorno

Definidas en `infra/.env` (ver plantilla en `infra/.env.example`):

| Variable            | Descripción                    |
|---------------------|--------------------------------|
| `POSTGRES_USER`     | Usuario de la base de datos    |
| `POSTGRES_PASSWORD` | Contraseña de la base de datos |
| `POSTGRES_DB`       | Nombre de la base de datos     |

## Estructura

```
.
├── infra/                 # Infraestructura local (Docker Compose)
│   ├── docker-compose.yml
│   └── .env.example
└── README.md
```

Los datos persisten en volúmenes de Docker (`abm-db-data`, `abm-grafana-data`).

## Comandos útiles

```bash
docker compose up -d          # levantar en segundo plano
docker compose down           # parar (conserva los datos)
docker compose logs -f db     # ver logs de la base de datos
docker compose down -v        # parar y BORRAR los volúmenes (¡borra los datos!)
```
