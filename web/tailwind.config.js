/** @type {import('tailwindcss').Config} */

// eslint-disable-next-line no-undef
module.exports = {
  prefix: "tw-",
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts}", // path to all template files
  ],
  theme: {
    extend: {
      colors: {
        primary: {
          DEFAULT: "#5960B2",
        },
        secondary: {
          DEFAULT: "#5ca380",
        },
      },
    },
  },
  plugins: [],
};
