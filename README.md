# Shine Backend

Backend service for **Shine a.s.d.**, built with **Django** and **Django REST Framework**.

This backend is the single source of truth for:

* users and roles (admin, instructor, parent)
* courses and schedules
* attendance tracking
* payments
* documents

It exposes a REST API consumed by:

* the web admin panel
* the parent portal
* the instructor mobile/web app

---

## Tech Stack

* Python 3.14
* Django
* Django REST Framework
* PostgreSQL (production)
* SQLite (development)

---

## Project Structure

```
parkourshine-backend/
├── config/                 # Django project configuration
│   ├── settings/
│   │   ├── base.py
│   │   ├── dev.py
│   │   └── prod.py
│   ├── urls.py
│   └── asgi.py / wsgi.py
├── apps/                   # Domain apps
│   ├── users/
│   ├── courses/
│   ├── attendance/
│   ├── payments/
│   └── documents/
├── manage.py
├── requirements.txt
├── .env.example
├── .gitignore
└── README.md
```

---

## Requirements

* Python 3.10+ recommended
* pip
* virtualenv (or equivalent)

---

## Local Development Setup

### 1. Clone the repository

```bash
git clone <repository-url>
cd parkourshine-backend
```

### 2. Create and activate a virtual environment

```bash
python -m venv venv
source venv/bin/activate  # macOS / Linux
venv\\Scripts\\activate   # Windows
```

### 3. Install dependencies

```bash
pip install -r requirements.txt
```

### 4. Environment variables

Create a `.env` file based on `.env.example`:

```bash
cp .env.example .env
```

Edit the values as needed for local development.

---

## Database

### Development

* Uses **SQLite** by default
* Database file: `db.sqlite3`
* No manual setup required

### Apply migrations

```bash
python manage.py migrate
```

---

## Create a Superuser (Admin)

```bash
python manage.py createsuperuser
```

This user can access the Django admin panel.

---

## Run the Development Server

```bash
python manage.py runserver
```

Server will be available at:

```
http://127.0.0.1:8000/
```

Admin panel:

```
http://127.0.0.1:8000/admin/
```

---

## API Development

This project uses **Django REST Framework**.

Typical flow:

1. Define models in `apps/*/models.py`
2. Create serializers
3. Create API views or viewsets
4. Register routes in `urls.py`

All business logic should live in the backend. Frontend clients must treat the API as the source of truth.

---

## Migrations

When models change:

```bash
python manage.py makemigrations
python manage.py migrate
```

⚠️ Always review generated migrations before applying them, especially when data already exists.

---

## Environments

* **Development**: SQLite, debug enabled
* **Production**: PostgreSQL, debug disabled

Environment-specific settings are located in:

```
config/settings/
```

---

## Production Notes

* Do not use SQLite in production
* Use PostgreSQL
* Enable automatic database backups
* Never commit `.env` files
* Never modify applied migrations in production

---

## Version Control Rules

* `main` → production-ready code
* `develop` → active development
* feature branches for new functionality

---

## License

Private project – all rights reserved.
