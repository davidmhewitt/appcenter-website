# AppCenter Website

This is the code behind the AppCenter website, including the app browser and the developer dashboard. It is separated into a Rust backend and a NextJS/Tailwind frontend. These can be worked on separately, see the individual `frontend` and `backend` folder's README files for details.

## Development

For hacking on the frontend with dummy data, see the README in the `frontend/` folder.

Otherwise, to run both the backend and frontend together, you will first need to define some secrets in a `.env` file. We can generate the mandatory ones with the following:

```
echo APP_SECRET__SECRET_KEY=\'`openssl rand 32 | openssl enc -A -base64`\' >> .env
echo APP_SECRET__HMAC_SECRET=\'`openssl rand 64 | openssl enc -A -base64`\' >> .env
echo PG_PASSWORD=\'`openssl rand -base64 20`\' >> .env
```

If you want to test the git integration (GitHub login or submitting apps), you will need to define some GitHub secrets in the `.env` file:

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


Once the secrets are defined, you can use the Docker Compose file in the root of the repository:

```
docker-compose up
```