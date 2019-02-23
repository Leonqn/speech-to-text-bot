using System;
using System.IO;
using Microsoft.AspNetCore.Hosting;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;

namespace SpeechRecognitionService
{
    public class SpeechRecognitionSamples
    {
        public static void Main(string[] args)
        {
            var port =
                args.Length == 1
                    ? int.Parse(args[0])
                    : 80;
            new WebHostBuilder()
            .UseKestrel(opt => {
                opt.Limits.MaxRequestBodySize = null;
            })
            .ConfigureAppConfiguration((_, config) =>
            {
                config.AddIniFile("config.ini", true);
                config.AddEnvironmentVariables();
            })
            .ConfigureLogging((_, config) =>
            {
                config.AddConsole();
                config.SetMinimumLevel(LogLevel.Information);
            })
            .UseUrls($"http://*:{port}/")
            .UseStartup<Startup>()
            .Build()
            .Run();

        }
    }
}
