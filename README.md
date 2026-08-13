# shine-backend

Backend service for the Shine Parkour association. 

It is built as a single Rust monolith that handles the public website, the internal management portal, and the REST APIs for the instructor app.

## Project Scope

* **Public Website**: Server-side rendered (SSR) pages using Askama templates and HTMX for light frontend interactivity.
* **Admin Portal**: Internal dashboard for managing member enrollments, payments, and medical certificates (uploaded to S3).
* **Instructor API**: JSON REST endpoints used by the dedicated mobile app to sync offline attendance and access emergency student contacts.

## Tech Stack

* **Language**: Rust
* **Framework**: Axum
* **Templating**: Askama (compile-time validated HTML)
* **Frontend**: HTMX & Plain CSS
* **Database**: SQLite (via SQLx)
* **File Storage**: Hetzner S3 Object Storage
