import preset from '@sesame/shared/tailwind.preset.js';

/** @type {import('tailwindcss').Config} */
export default {
  presets: [preset],
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}', '../shared/src/**/*.{js,ts,jsx,tsx}'],
};
