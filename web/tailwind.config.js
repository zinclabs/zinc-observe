/** @type {import('tailwindcss').Config} */

// eslint-disable-next-line no-undef
export default {
  prefix: "tw-",
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts}", // path to all template files
  ],
  theme: {
    extend: {
      colors: {
        primary: {
          DEFAULT: "rgb(var(--q-primary) / <alpha-value>)",
        },
        secondary: {
          DEFAULT: "rgb(var(--q-secondary) / <alpha-value>)",
        },
      },
    },
  },
  plugins: [require("@tailwindcss/typography")],
};
