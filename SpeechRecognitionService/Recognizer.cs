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

        public async Task<string> Recognize(string language, Stream audio, TimeSpan timeout)
        {
            var config = SpeechConfig.FromSubscription(subscriptionKey, region);
            config.SpeechRecognitionLanguage = language;
            var recognitionStream = await CreatePushStreamAsync(audio);
            if (recognitionStream == null)
                return null;
            try
            {
                await maxConcurrentRequests.WaitAsync();
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
                    recognizer.SessionStopped += (_, e) =>
                    {
                        tcs.TrySetException(new Exception("Unknown error has occurred"));
                    };

                    await recognizer.StartContinuousRecognitionAsync();

                    var timeout_task = Task.Delay(timeout);
                    var response_or_timeout = await Task.WhenAny(timeout_task, tcs.Task);
                    await recognizer.StopContinuousRecognitionAsync();

                    var response =
                        response_or_timeout is Task<string> s
                            ? await s
                            : throw new Exception("timeout");

                    return response;
                }
            }
            finally
            {
                maxConcurrentRequests.Release();
            }
        }

        private async Task<PushAudioInputStream> CreatePushStreamAsync(Stream stream)
        {
            var read = 0;
            var recognitionStream = AudioInputStream.CreatePushStream();
            var buffer = ArrayPool<byte>.Shared.Rent(80000);
            var sumRead = 0;
            try
            {
                while ((read = await stream.ReadAsync(buffer, 0, buffer.Length)) != 0)
                {
                    sumRead += read;
                    recognitionStream.Write(buffer, read);
                }
                recognitionStream.Close();
                if (sumRead == 0)
                    return null;
                return recognitionStream;
            }
            finally
            {
                ArrayPool<byte>.Shared.Return(buffer);
            }

        }
    }
}
