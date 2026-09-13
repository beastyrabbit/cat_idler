# syntax=docker/dockerfile:1.7
FROM mcr.microsoft.com/dotnet/sdk:10.0.400 AS build
WORKDIR /src
COPY global.json ./
COPY server ./server
COPY unity/Assets/Forest/Simulation ./unity/Assets/Forest/Simulation
COPY unity/Assets/Forest/Authority ./unity/Assets/Forest/Authority
RUN dotnet restore server/Forest.Server/Forest.Server.csproj --locked-mode
RUN dotnet publish server/Forest.Server/Forest.Server.csproj --no-restore -c Release -o /out -warnaserror

FROM mcr.microsoft.com/dotnet/aspnet:10.0
WORKDIR /app
COPY --from=build /out ./
USER root
RUN mkdir /data && chown app:app /data
USER app
ENV FOREST_LISTEN=http://0.0.0.0:8788 \
    FOREST_SAVE_PATH=/data/world-v1.json
VOLUME ["/data"]
EXPOSE 8788
# A public listener requires SESSION_HMAC_SECRET through the approved runtime
# secret mechanism. No key, world or identity is copied into this image.
ENTRYPOINT ["dotnet", "Forest.Server.dll"]
