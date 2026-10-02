using System;
using System.Collections;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text;
using IdleCatForest.Simulation;
using Newtonsoft.Json.Linq;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Scripted player for `forest.sh capture`. It exists only when the player starts with
    /// `--forest-capture-script <script.json> --forest-capture-out <dir>`, and drives the game through public
    /// members only (plan section 3.7). The script is a JSON array; each step is a string ("focusVillage",
    /// "close", "quit") or an object with one property: wait, speed, focusVillage, zoom, level, selectCat,
    /// selectBuilding, open, close, act, shot, quit. Every step is logged to capture-log.txt in the output folder.
    /// </summary>
    public sealed class CaptureDriver : MonoBehaviour
    {
        const string ScriptArgument = "--forest-capture-script";
        const string OutArgument = "--forest-capture-out";
        const string SelectedCatToken = "$selectedCat";
        const string SelectedBuildingToken = "$selectedBuilding";

        string scriptPath = "", outDirectory = "", logPath = "";
        string selectedCatId = "", selectedBuildingId = "";
        int failures;
        float started;

        [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.AfterSceneLoad)]
        static void Bootstrap()
        {
            var args = Environment.GetCommandLineArgs();
            string script = Argument(args, ScriptArgument);
            if (script == null) return;
            var host = new GameObject("CaptureDriver");
            DontDestroyOnLoad(host);
            var driver = host.AddComponent<CaptureDriver>();
            driver.scriptPath = script;
            driver.outDirectory = Argument(args, OutArgument) ?? Path.Combine(Path.GetDirectoryName(Path.GetFullPath(script)) ?? ".", "capture");
        }

        static string Argument(string[] args, string key)
        {
            int at = Array.IndexOf(args, key);
            return at >= 0 && at + 1 < args.Length ? args[at + 1] : null;
        }

        IEnumerator Start()
        {
            Application.runInBackground = true;
            started = Time.realtimeSinceStartup;
            Directory.CreateDirectory(outDirectory);
            logPath = Path.Combine(outDirectory, "capture-log.txt");
            File.WriteAllText(logPath, "");
            Log("capture start script=" + scriptPath + " out=" + outDirectory + " screen=" + Screen.width + "x" + Screen.height);

            JArray steps;
            try { steps = JArray.Parse(File.ReadAllText(scriptPath)); }
            catch (Exception e) { Log("FAIL script could not be read: " + e.Message); Quit(2); yield break; }

            float deadline = Time.realtimeSinceStartup + 120;
            while (ForestGame.Instance == null || ForestGame.Instance.CurrentWorld == null)
            {
                if (Time.realtimeSinceStartup > deadline) { Log("FAIL the world did not open within 120 s"); Quit(1); yield break; }
                yield return null;
            }
            for (int i = 0; i < 10; i++) yield return null;
            Log("world ready");

            for (int index = 0; index < steps.Count; index++)
            {
                string op; JToken value;
                if (!Describe(steps[index], out op, out value))
                {
                    Log("FAIL step " + (index + 1) + " is not a known step: " + steps[index].ToString(Newtonsoft.Json.Formatting.None));
                    Quit(2); yield break;
                }
                string label = "step " + (index + 1) + " " + op + (value == null || value.Type == JTokenType.Null ? "" : " " + value.ToString(Newtonsoft.Json.Formatting.None));
                if (op == "quit") { Log(label); break; }
                if (op == "wait")
                {
                    float seconds = Number(value, 0);
                    Log(label);
                    yield return new WaitForSecondsRealtime(Mathf.Max(0, seconds));
                    continue;
                }
                if (op == "shot")
                {
                    yield return new WaitForEndOfFrame();
                    Log(label + " -> " + Shot(value?.ToString() ?? "shot"));
                    continue;
                }
                string result;
                try { result = Run(op, value); }
                catch (Exception e) { result = "FAIL " + e.GetType().Name + ": " + e.Message; }
                if (result.StartsWith("FAIL", StringComparison.Ordinal)) failures++;
                Log(label + " -> " + result);
                yield return null;
            }
            Log("capture done failures=" + failures);
            Quit(failures == 0 ? 0 : 1);
        }

        static bool Describe(JToken step, out string op, out JToken value)
        {
            op = ""; value = null;
            if (step.Type == JTokenType.String) op = step.ToString();
            else if (step is JObject obj && obj.Count == 1) { var property = obj.Properties().First(); op = property.Name; value = property.Value; }
            else return false;
            switch (op)
            {
                case "wait": case "speed": case "focusVillage": case "zoom": case "level": case "selectCat":
                case "selectBuilding": case "open": case "close": case "act": case "shot": case "quit":
                    return true;
                default:
                    return false;
            }
        }

        string Run(string op, JToken value)
        {
            var game = ForestGame.Instance;
            var view = game.GetComponent<ForestView>();
            switch (op)
            {
                case "speed":
                    game.SetSpeed(Number(value, 1));
                    return "ok";
                case "focusVillage":
                    view.FocusVillage();
                    return "ok";
                case "zoom":
                    view.ZoomBy(Number(value, 0));
                    return "ok";
                case "level":
                    view.ViewLevel((int)Math.Round(Number(value, 0)));
                    return "ok";
                case "selectCat":
                    return SelectCat(game, view, value?.ToString() ?? "firstWorking");
                case "selectBuilding":
                    return SelectBuilding(game, view, value?.ToString() ?? "");
                case "open":
                    game.UI.OpenPanel(value?.ToString() ?? "");
                    return "ok section=" + game.UI.OpenSection;
                case "close":
                    game.UI.ClosePanel();
                    return "ok";
                case "act":
                    return Act(game, value);
                default:
                    return "FAIL unknown step";
            }
        }

        string SelectCat(ForestGame game, ForestView view, string wanted)
        {
            var village = game.Selected;
            if (village == null) return "FAIL no village selected";
            Cat cat;
            if (wanted == "firstWorking")
            {
                cat = village.Cats.FirstOrDefault(c => c.Alive && c.JobId != "");
                if (cat == null) { cat = village.Cats.FirstOrDefault(c => c.Alive); if (cat == null) return "FAIL no living cat"; view.InspectCat(cat.Id); selectedCatId = cat.Id; selectedBuildingId = ""; return "ok " + cat.Id + " (" + cat.Name + ", nobody is working yet)"; }
            }
            else if (wanted == "first") cat = village.Cats.FirstOrDefault(c => c.Alive);
            else cat = village.Cats.FirstOrDefault(c => c.Id == wanted);
            if (cat == null) return "FAIL no cat matches '" + wanted + "'";
            view.InspectCat(cat.Id);
            selectedCatId = cat.Id; selectedBuildingId = "";
            return "ok " + cat.Id + " (" + cat.Name + ")";
        }

        string SelectBuilding(ForestGame game, ForestView view, string wanted)
        {
            var village = game.Selected;
            if (village == null) return "FAIL no village selected";
            var building = village.Buildings.FirstOrDefault(b => b.Id == wanted)
                ?? village.Buildings.FirstOrDefault(b => string.Equals(b.Kind, wanted, StringComparison.OrdinalIgnoreCase));
            if (building == null) return "FAIL no building matches '" + wanted + "'";
            view.InspectBuilding(building.Id);
            selectedBuildingId = building.Id; selectedCatId = "";
            return "ok " + building.Id + " (" + building.Kind + ")";
        }

        string Act(ForestGame game, JToken value)
        {
            if (!(value is JObject template)) return "FAIL act needs a GameAction object";
            var action = (JObject)template.DeepClone();
            foreach (var property in action.Properties().ToList())
            {
                if (property.Value.Type != JTokenType.String) continue;
                string text = property.Value.ToString();
                if (text == SelectedCatToken) { if (selectedCatId == "") return "FAIL " + SelectedCatToken + " used before selectCat"; property.Value = selectedCatId; }
                else if (text == SelectedBuildingToken) { if (selectedBuildingId == "") return "FAIL " + SelectedBuildingToken + " used before selectBuilding"; property.Value = selectedBuildingId; }
            }
            var gameAction = JsonUtility.FromJson<GameAction>(action.ToString(Newtonsoft.Json.Formatting.None));
            if (string.IsNullOrEmpty(gameAction?.Kind)) return "FAIL act needs a Kind";
            game.Act(gameAction);
            return "sent " + gameAction.Kind;
        }

        string Shot(string name)
        {
            var safe = new StringBuilder();
            foreach (char ch in name) safe.Append(char.IsLetterOrDigit(ch) || ch == '-' || ch == '_' ? ch : '-');
            if (safe.Length == 0) safe.Append("shot");
            string path = Path.Combine(outDirectory, safe + ".png");
            Texture2D texture = null;
            try
            {
                texture = ScreenCapture.CaptureScreenshotAsTexture();
                File.WriteAllBytes(path, texture.EncodeToPNG());
                return "ok " + Path.GetFileName(path) + " " + texture.width + "x" + texture.height;
            }
            catch (Exception e) { failures++; return "FAIL " + e.GetType().Name + ": " + e.Message; }
            finally { if (texture != null) Destroy(texture); }
        }

        static float Number(JToken value, float fallback)
        {
            if (value == null) return fallback;
            if (value.Type == JTokenType.Integer || value.Type == JTokenType.Float) return value.Value<float>();
            return float.TryParse(value.ToString(), NumberStyles.Float, CultureInfo.InvariantCulture, out float parsed) ? parsed : fallback;
        }

        void Log(string message)
        {
            var world = ForestGame.Instance != null ? ForestGame.Instance.CurrentWorld : null;
            string line = "[real " + (Time.realtimeSinceStartup - started).ToString("F1", CultureInfo.InvariantCulture) + "s | sim "
                + (world != null ? world.TimeSeconds.ToString("F1", CultureInfo.InvariantCulture) + "s" : "-") + "] " + message;
            Debug.Log("[capture] " + line);
            if (logPath != "") File.AppendAllText(logPath, line + Environment.NewLine);
        }

        static void Quit(int code) => Application.Quit(code);
    }
}
