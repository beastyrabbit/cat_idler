using System;
using System.Collections;
using System.Globalization;
using System.IO;
using System.Linq;
using Newtonsoft.Json.Linq;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Evidence tool for camera and picking captures. It exists only when the player starts with
    /// <c>--forest-input-replay &lt;steps.json&gt;</c> or the <c>FOREST_INPUT_REPLAY</c> environment variable
    /// (so <c>forest.sh capture</c> can pass it through) and feeds the
    /// input router the gestures a player would make: left drags, intents and world clicks. It drives
    /// the router's dispatch API, not the devices, so a stray real mouse cannot disturb a capture.
    /// Steps are a JSON array of objects, each with "at" (seconds after the world opened) and one of:
    /// "drag": [x0, y0, x1, y1] in screen fractions (with optional "seconds"), "intent": an
    /// <see cref="InputIntent"/> name, "clickRoof": a building kind, "clickCat": "leader".
    /// A click is dispatched as a PrimaryClick; the replay selects what <see cref="ForestView.SubjectAt"/>
    /// returns the way the selection owner does. Results go to input-replay-log.txt.
    /// </summary>
    public sealed class InputReplay : MonoBehaviour
    {
        private const string ReplayArgument = "--forest-input-replay";
        private const string ReplayVariable = "FOREST_INPUT_REPLAY";
        private const string OutArgument = "--forest-capture-out";
        private string stepsPath = "", logPath = "";

        [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.AfterSceneLoad)]
        private static void Bootstrap()
        {
            var args = Environment.GetCommandLineArgs();
            int at = Array.IndexOf(args, ReplayArgument);
            string path = at >= 0 && at + 1 < args.Length ? args[at + 1] : Environment.GetEnvironmentVariable(ReplayVariable);
            if (string.IsNullOrEmpty(path) || Application.isEditor) return;
            var host = new GameObject("InputReplay");
            DontDestroyOnLoad(host);
            var replay = host.AddComponent<InputReplay>();
            replay.stepsPath = path;
            int outAt = Array.IndexOf(args, OutArgument);
            string outDirectory = outAt >= 0 && outAt + 1 < args.Length ? args[outAt + 1] : Path.GetDirectoryName(Path.GetFullPath(replay.stepsPath)) ?? ".";
            Directory.CreateDirectory(outDirectory);
            replay.logPath = Path.Combine(outDirectory, "input-replay-log.txt");
        }

        private IEnumerator Start()
        {
            File.WriteAllText(logPath, "");
            JArray steps;
            try { steps = JArray.Parse(File.ReadAllText(stepsPath)); }
            catch (Exception e) { Log("FAIL steps could not be read: " + e.Message); yield break; }
            while (ForestGame.Instance == null || ForestGame.Instance.CurrentWorld == null || InputRouter.Current == null) yield return null;
            for (int i = 0; i < 10; i++) yield return null;
            float opened = Time.realtimeSinceStartup;
            Log("world ready");
            foreach (var token in steps.OfType<JObject>().OrderBy(s => s.Value<float?>("at") ?? 0))
            {
                float at = token.Value<float?>("at") ?? 0;
                while (Time.realtimeSinceStartup - opened < at) yield return null;
                if (token["drag"] is JArray drag && drag.Count == 4)
                {
                    float seconds = token.Value<float?>("seconds") ?? .8f;
                    yield return Drag(Fraction(drag[0], drag[1]), Fraction(drag[2], drag[3]), seconds);
                }
                else if (token["intent"] != null)
                {
                    bool parsed = Enum.TryParse(token.Value<string>("intent"), out InputIntent intent);
                    Log("intent " + token.Value<string>("intent") + " -> " + (parsed ? InputRouter.Current.Dispatch(intent).ToString() : "FAIL unknown intent"));
                }
                else if (token["clickRoof"] != null) Click(RoofOf(token.Value<string>("clickRoof")), "roof of " + token.Value<string>("clickRoof"));
                else if (token["clickCat"] != null) Click(LeaderBody(), "leader");
                else Log("FAIL unknown step " + token.ToString(Newtonsoft.Json.Formatting.None));
            }
            Log("replay done");
        }

        private IEnumerator Drag(Vector2 from, Vector2 to, float seconds)
        {
            var router = InputRouter.Current;
            var view = ForestGame.Instance.View;
            var before = view.CameraFocus;
            router.DispatchPointer(PointerAction.PrimaryDragStart, new PointerInfo(from, Vector2.zero, from, false, false));
            float start = Time.realtimeSinceStartup;
            var last = from;
            while (Time.realtimeSinceStartup - start < seconds)
            {
                float t = Mathf.Clamp01((Time.realtimeSinceStartup - start) / seconds);
                var position = Vector2.Lerp(from, to, t * t * (3 - 2 * t));
                router.DispatchPointer(PointerAction.PrimaryDrag, new PointerInfo(position, position - last, from, false, false));
                last = position;
                yield return null;
            }
            router.DispatchPointer(PointerAction.PrimaryDragEnd, new PointerInfo(to, to - last, from, false, false));
            Log("drag " + from + " -> " + to + " moved the framed point " + before + " -> " + view.CameraFocus);
        }

        private void Click(Vector2? screen, string label)
        {
            if (screen == null) { Log("FAIL click " + label + ": nothing to click"); return; }
            var view = ForestGame.Instance.View;
            Subject picked = Subject.None;
            using (InputRouter.Current.BindPointer(PointerAction.PrimaryClick, InputPriority.Modal + 1, p => { picked = view.SubjectAt(p.Position); return true; }))
                InputRouter.Current.DispatchPointer(PointerAction.PrimaryClick, new PointerInfo(screen.Value, Vector2.zero, screen.Value, false, false));
            if (picked.Kind == SubjectKind.Building) view.InspectBuilding(picked.Id);
            else if (picked.Kind == SubjectKind.Cat) view.InspectCat(picked.Id);
            Log("click " + label + " at " + screen.Value.ToString("F0") + " picked " + picked);
        }

        private static Vector2? RoofOf(string kind)
        {
            var game = ForestGame.Instance;
            var building = game.Selected.Buildings.FirstOrDefault(b => string.Equals(b.Kind, kind, StringComparison.OrdinalIgnoreCase));
            if (building == null || !game.View.TryGetEntity("building:" + building.Id, out var go)) return null;
            var renderers = go.GetComponentsInChildren<Renderer>();
            if (renderers.Length == 0) return null;
            var bounds = renderers[0].bounds; foreach (var r in renderers) bounds.Encapsulate(r.bounds);
            return game.View.Camera.WorldToScreenPoint(new Vector3(bounds.center.x, bounds.max.y - .05f, bounds.center.z));
        }

        private static Vector2? LeaderBody()
        {
            var game = ForestGame.Instance;
            if (!game.View.TryGetEntity("cat:" + game.Selected.LeaderId, out var go)) return null;
            return game.View.Camera.WorldToScreenPoint(go.transform.position + Vector3.up * .28f);
        }

        private static Vector2 Fraction(JToken x, JToken y) => new Vector2(x.Value<float>() * Screen.width, y.Value<float>() * Screen.height);

        private void Log(string message)
        {
            var world = ForestGame.Instance != null ? ForestGame.Instance.CurrentWorld : null;
            string line = "[real " + Time.realtimeSinceStartup.ToString("F1", CultureInfo.InvariantCulture) + "s | sim "
                + (world != null ? world.TimeSeconds.ToString("F1", CultureInfo.InvariantCulture) + "s" : "-") + "] " + message;
            Debug.Log("[input-replay] " + line);
            if (logPath != "") File.AppendAllText(logPath, line + Environment.NewLine);
        }
    }
}
