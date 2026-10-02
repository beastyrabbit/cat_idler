using System;

namespace IdleCatForest.Presentation
{
    public enum SubjectKind { None, Cat, Building, Pile, Expedition, Site, Entrance }

    /// <summary>What the player has selected or is hovering: a kind plus the simulation id.</summary>
    public readonly struct Subject : IEquatable<Subject>
    {
        public static readonly Subject None = default;

        public readonly SubjectKind Kind;
        public readonly string Id;

        public Subject(SubjectKind kind, string id)
        {
            Kind = kind;
            Id = id ?? "";
        }

        public bool IsNone => Kind == SubjectKind.None;

        public bool Equals(Subject other) => Kind == other.Kind && string.Equals(Id ?? "", other.Id ?? "", StringComparison.Ordinal);
        public override bool Equals(object obj) => obj is Subject other && Equals(other);
        public override int GetHashCode() => ((int)Kind * 397) ^ StringComparer.Ordinal.GetHashCode(Id ?? "");
        public static bool operator ==(Subject a, Subject b) => a.Equals(b);
        public static bool operator !=(Subject a, Subject b) => !a.Equals(b);
        public override string ToString() => IsNone ? "None" : Kind + ":" + Id;
    }
}
