using System;
using System.Buffers;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using Microsoft.CognitiveServices.Speech;
using Microsoft.CognitiveServices.Speech.Audio;

namespace SpeechRecognitionService
{
    public class Recognizer
    {
        private readonly string subscriptionKey;
        private readonly string region;
        private readonly SemaphoreSlim maxConcurrentRequests;

        public Recognizer(string subscriptionKey, string region, int maxConcurrentRequests)
        {
            this.subscriptionKey = subscriptionKey;
            this.region = region;
            this.maxConcurrentRequests = new SemaphoreSlim(maxConcurrentRequests);
        }

        public async Task<string> Recognize(string language, Stream audio)
        {
            var config = SpeechConfig.FromSubscription(subscriptionKey, region);
            config.SpeechRecognitionLanguage = language;

            var buffer = ArrayPool<byte>.Shared.Rent(80000);
            var read = 0;
            var recognitionStream = AudioInputStream.CreatePushStream();
            while ((read = await audio.ReadAsync(buffer, 0, buffer.Length)) != 0)
            {
                recognitionStream.Write(buffer, read);
            }
            recognitionStream.Close();
            
            await maxConcurrentRequests.WaitAsync();
            try
            {
                using (var recognizer = new SpeechRecognizer(config, AudioConfig.FromStreamInput(recognitionStream)))
                {
                    var tcs = new TaskCompletionSource<string>();
                    recognizer.Recognized += (_, e) =>
                    {
                        if (e.Result.Reason == ResultReason.RecognizedSpeech)
                        {
                            tcs.TrySetResult(e.Result.Text);
                        }
                        if (e.Result.Reason == ResultReason.NoMatch)
                        {
                            tcs.TrySetResult(null);
                        }
                    };
                    recognizer.Canceled += (_, e) =>
                    {
                        if (e.Reason == CancellationReason.Error)
                        {
                            tcs.TrySetException(new Exception(e.ErrorDetails));
                        }
                    };

                    await recognizer.StartContinuousRecognitionAsync();

                    var response = await tcs.Task;

                    await recognizer.StopContinuousRecognitionAsync();
                    return response;
                }
            }
            finally
            {
                maxConcurrentRequests.Release();
            }
        }
    }
}
