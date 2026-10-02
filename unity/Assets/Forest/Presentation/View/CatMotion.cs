using System;
using System.Linq;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>Procedural part animation of one rendered cat (paws, tail, head, body).</summary>
    public sealed class CatMotion : MonoBehaviour
    {
        /// <summary>
        /// A paw touched the ground: the cat's simulation id and the contact point in world space.
        /// Declared for footstep sounds and dust; nothing raises it yet.
        /// </summary>
#pragma warning disable 0067
        public static event Action<string, Vector3> PawContact;
#pragma warning restore 0067

        public string CatId = "";
        private Transform[] parts;
        private Quaternion[] rest;
        private Vector3[] positions;
        private float phase;

        public void Animate(bool walking, string goal)
        {
            if (parts == null)
            {
                parts = GetComponentsInChildren<Transform>().Where(t => t.name.StartsWith("Paw") || t.name == "Tail" || t.name == "Head" || t.name == "Body").ToArray();
                rest = parts.Select(t => t.localRotation).ToArray(); positions = parts.Select(t => t.localPosition).ToArray();
                uint hash = 2166136261; foreach (char c in CatId) hash = unchecked((hash ^ c) * 16777619);
                phase = hash % 628 / 100f;
            }
            float time = Time.time;
            bool sleeping = goal == "need_sleep" && !walking;
            bool working = !walking && (goal.Contains("work") || goal.Contains("harvest") || goal.Contains("produce"));
            for (int i = 0; i < parts.Length; i++)
            {
                bool opposite = parts[i].name == "PawFrontRight" || parts[i].name == "PawBackLeft";
                float angle = parts[i].name.StartsWith("Paw") ? (walking ? Mathf.Sin(time * 11 + phase + (opposite ? Mathf.PI : 0)) * 26 : working && parts[i].name.StartsWith("PawFront") ? Mathf.Sin(time * 5 + phase) * 12 : sleeping ? -28 : 0)
                    : parts[i].name == "Tail" ? Mathf.Sin(time * 1.6f + phase) * (sleeping ? 3 : 12)
                    : parts[i].name == "Head" ? sleeping ? 22 : working ? 10 + Mathf.Sin(time * 5 + phase) * 6 : Mathf.Sin(time * .8f + phase) * 4 : 0;
                float glance = parts[i].name == "Head" && !walking && !sleeping && !working ? Mathf.Sin(time * .45f + phase) * 12 : 0;
                parts[i].localRotation = rest[i] * Quaternion.Euler(angle, glance, 0);
                parts[i].localPosition = positions[i] + (parts[i].name == "Body" ? Vector3.up * (walking ? Mathf.Sin(time * 22 + phase) * .012f : Mathf.Sin(time * 2 + phase) * .007f) : Vector3.zero);
            }
        }
    }
}
