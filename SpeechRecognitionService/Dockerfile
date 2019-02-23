FROM microsoft/dotnet:2.2-sdk-bionic AS build

WORKDIR /app
COPY . .

RUN dotnet restore
RUN dotnet publish -c Release -o out


FROM microsoft/dotnet:2.2-aspnetcore-runtime-bionic AS runtime

WORKDIR /app

RUN apt-get update
RUN apt-get install libssl1.0.0 libasound2 wget
COPY --from=build /app/out ./
ENTRYPOINT ["dotnet", "SpeechRecognitionService.dll"]