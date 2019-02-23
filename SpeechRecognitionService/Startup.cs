using System;
using System.IO;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;

namespace SpeechRecognitionService
{
    public class Startup
    {
        public void Configure(IApplicationBuilder app, IConfiguration config, ILogger<Startup> logger)
        {
            var key = config.GetValue<string>("azure_speech_key");
            var region = config.GetValue<string>("azure_speech_region");
            var maxConcurrentRequests = config.GetValue("azure_speech_max_concurrency", 1);
            var recognizer = new Recognizer(key, region, maxConcurrentRequests);
            app.Map("/api/recognize/audio", builder =>
            {
                builder.Run(async context =>
                {
                    if (context.Request.Method == "POST")
                    {
                        context.Request.Query.TryGetValue("lang", out var lang);
                        try
                        {
                            var recognized = await recognizer.Recognize(lang.ToString(), context.Request.Body, TimeSpan.FromMinutes(2));
                            await context.Response.WriteAsync(recognized ?? "");
                        }
                        catch (Exception e)
                        {
                            logger.LogError(e, "error occured");
                            context.Response.StatusCode = StatusCodes.Status500InternalServerError;
                            await context.Response.WriteAsync(e.Message);
                        }
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
