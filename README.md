# AppCenter Website

This is the WIP code behind the (hopefully) new AppCenter website, including the app browser and the developer dashboard. It is separated into a Rust backend and a NextJS/Tailwind frontend.

## Running

Set up a development environment using one of the methods below. Then either:

### Run the Frontend (With Mock Data)

In multiple terminal tabs/windows:

```
cd frontend
npm ci
npm run mocks
```

```
cd frontend
npm run dev
```

Visit http://localhost:3000

### Run the Frontend (with real data from the backend)

In multiple terminal windows:

```
cd backend
cargo run -p background-worker
```

```
cd backend
cargo run
```

```
cd frontend
npm ci
npm run dev
```

Visit http://localhost:3000

## Setting up a development environment (the easy way)

The recommended way to set up a development environment is to use the devcontainer config for VSCode (or any other editor that supports the devcontainer spec). When prompted whether to use the "Local" or "Remote" config, use "Local". The "Remote" config is for use with GitHub codespaces. This will build a containerised development environment with the necessary Rust and Node toolchains and the required Postgres and Redis services.

### Setting up GitHub integration

If you want to test the GitHub integration (GitHub login or submitting apps), you will need to define some GitHub secrets in the `backend/.env` file:

```
APP_GITHUB__CLIENT_ID=
APP_GITHUB__CLIENT_SECRET=
APP_GITHUB__USERNAME=
APP_GITHUB__ACCESS_TOKEN=
APP_GITHUB__REVIEWS_URL=
```

- `APP_GITHUB__CLIENT_ID` and `APP_GITHUB__CLIENT_SECRET` are the values provided by GitHub when setting up an OAuth app.
- `APP_GITHUB__USERNAME` is the GitHub username of the account that should be used to push commits and open PRs on the `appcenter-reviews` repository when submitting new apps. In production, this is `elementaryBot`.
- `APP_GITHUB__ACCESS_TOKEN` is a PAT for the `APP_GITHUB__USERNAME` account. It should have `public_repo` scope as a minimum.
- `APP_GITHUB__REVIEWS_URL` is the HTTPS url of the Git repository that will serve as the `appcenter-reviews` repository for submitting app PRs to. This can be a fork of https://github.com/elementary/appcenter-reviews for testing.

### Database Migrations

If you need to make any changes to the backend database schema, you will need to install `diesel` with `cargo install diesel`

## Setting up a development environment (the manual way)

### Requirements

- Rust >= 1.66 (at the time of writing, may be more in the future)
- Node >= 14 (but 19 recommended, as this is used for development)
- Redis server
- Postgres server

### Environment variables

In `backend/.env`, configure the following variables to appropriate values for your environment:

```
DATABASE_URL=postgresql://appcenter:appcenter@db/appcenter_website
APP_DATABASE__URL=postgresql://appcenter:appcenter@db/appcenter_website
APP_REDIS__URI=redis://redis
```

You may also want to configure the GitHub environment variables as described above.