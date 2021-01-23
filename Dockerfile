FROM node:13

RUN mkdir -p /usr/src/app

WORKDIR /usr/src/app

COPY package.json ./
RUN npm install

COPY . ./

CMD ["npm", "run", "serve"]
