using System.IO;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.Configuration;

namespace SpeechRecognitionService
{
    public class Startup
    {
        public void Configure(IApplicationBuilder app, IConfiguration config)
        {
            var key = config["azure_speech_key"];
            var region = config["azure_speech_region"];
            var maxConcurrentRequests = config.GetValue("azure_speech_max_concurrency", 1);
            var recognizer = new Recognizer(key, region, maxConcurrentRequests);
            app.Map("/api/recognize/audio", builder =>
            {
                builder.Run(async context =>
                {
                    if (context.Request.Method == "POST")
                    {
                        context.Request.Query.TryGetValue("lang", out var lang);
                        var recognized = await recognizer.Recognize(lang.ToString() ?? "ru-RU", context.Request.Body);
                        await context.Response.WriteAsync(recognized ?? "");
                    }
                    else
                    {
                        context.Response.StatusCode = StatusCodes.Status405MethodNotAllowed;
                    }
                });
            });
        }
    }
}
