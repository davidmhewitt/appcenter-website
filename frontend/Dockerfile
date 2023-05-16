FROM node:18

WORKDIR /app

COPY . ./

RUN npm ci

CMD npm run build && npm run start
