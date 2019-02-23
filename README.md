# Speech to text telegram bot.
Speech to text telegram bot. It can convert voice and video note messages to text. For speech recognition it uses [azure speech to text service](https://azure.microsoft.com/en-us/services/cognitive-services/speech-to-text/)


To run bot you should specify some required environment variables to docker-compose:
- [bot_api_key](https://github.com/Leonqn/speech-to-text-bot/blob/master/bot_config.env#L1) can be obtained through the [BotFather](https://telegram.me/botfather)
- [azure_speech_key](https://github.com/Leonqn/speech-to-text-bot/blob/master/speech_service_config.env#L1) and [azure_speech_region](https://github.com/Leonqn/speech-to-text-bot/blob/master/speech_service_config.env#L2) can be obtained through azure control panel

After specifying variables simply run
``` console
docker-compose up
```
