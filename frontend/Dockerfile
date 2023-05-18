FROM node:18

WORKDIR /app

COPY . ./

RUN npm ci

EXPOSE 3000

CMD npm run build && npm run start
