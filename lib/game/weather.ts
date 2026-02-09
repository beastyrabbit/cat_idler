import { normalizeSeed, rollSeeded } from "./seededRng";

// --- Types ---

export const WEATHER_TYPES = [
  "sunny",
  "rainy",
  "stormy",
  "foggy",
  "windy",
  "drought",
] as const;

export type WeatherType = (typeof WEATHER_TYPES)[number];

export interface WeatherModifiers {
  food: number;
  water: number;
  herbs: number;
}

export interface ForecastEntry {
  day: number;
  weather: WeatherType;
}

// --- Modifier Tables ---

const WEATHER_MODIFIER_TABLE: Record<WeatherType, WeatherModifiers> = {
  sunny: { food: 1.0, water: 1.0, herbs: 1.0 },
  rainy: { food: 0.9, water: 1.3, herbs: 1.0 },
  stormy: { food: 0.7, water: 1.5, herbs: 0.8 },
  foggy: { food: 0.8, water: 1.0, herbs: 1.1 },
  windy: { food: 0.9, water: 0.9, herbs: 0.9 },
  drought: { food: 1.1, water: 0.6, herbs: 1.0 },
};

// --- Functions ---

/**
 * Deterministic weather for a given seed + day number.
 * Chains the RNG twice (seed then day) for better entropy mixing.
 */
export function getWeather(seed: number, dayNumber: number): WeatherType {
  const normalDay = Math.abs(Math.floor(dayNumber));
  // First roll mixes the world seed
  const { nextSeed } = rollSeeded(normalizeSeed(seed) + normalDay);
  // Second roll mixes the day into the chain
  const { value } = rollSeeded(nextSeed + normalDay * 7919);
  const index = Math.floor(value * WEATHER_TYPES.length);
  return WEATHER_TYPES[index];
}

/**
 * Resource rate multipliers for a given weather type.
 */
export function getWeatherModifiers(weather: WeatherType): WeatherModifiers {
  return { ...WEATHER_MODIFIER_TABLE[weather] };
}

/**
 * Multi-day forecast starting from startDay.
 */
export function getWeatherForecast(
  seed: number,
  startDay: number,
  days: number,
): ForecastEntry[] {
  const forecast: ForecastEntry[] = [];
  for (let i = 0; i < days; i++) {
    const day = startDay + i;
    forecast.push({ day, weather: getWeather(seed, day) });
  }
  return forecast;
}
