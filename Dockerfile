# Imagen de la web de documentación de acm (Astro estático servido con nginx), no del intérprete.
# El contexto de build es la RAÍZ del repositorio: la web importa README.md y examples/.
#   docker build -t acm-docs .
#   docker run -p 8080:80 acm-docs

FROM node:22-alpine AS build
WORKDIR /repo/docs
COPY docs/package.json docs/package-lock.json ./
RUN npm ci
COPY README.md /repo/README.md
COPY examples /repo/examples
COPY docs ./
RUN npm run build

FROM nginx:1.27-alpine
COPY docs/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=build /repo/docs/dist /usr/share/nginx/html
EXPOSE 80
