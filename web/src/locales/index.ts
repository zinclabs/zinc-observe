// Copyright 2023 OpenObserve Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

import { createI18n } from "vue-i18n"; // import from runtime only
import { getLanguage } from "../utils/cookies";

// User defined lang
import zhLocale from "./languages/zh.json";
import enLocale from "./languages/en.json";
import trLocale from "./languages/tr.json";
import deLocale from "./languages/de.json";
import esLocale from "./languages/es.json";
import frLocale from "./languages/fr.json";
import itLocale from "./languages/it.json";
import jaLocale from "./languages/ja.json";
import koLocale from "./languages/ko.json";
import nlLocale from "./languages/nl.json";
import ptLocale from "./languages/pt.json";

const messages = {
  "en-gb": {
    ...enLocale,
  },
  "zh-cn": {
    ...zhLocale,
  },
  "tr-turk": {
    ...trLocale,
  },
  de: {
    ...deLocale,
  },
  es: {
    ...esLocale,
  },
  fr: {
    ...frLocale,
  },
  it: {
    ...itLocale,
  },
  ja: {
    ...jaLocale,
  },
  ko: {
    ...koLocale,
  },
  nl: {
    ...nlLocale,
  },
  pt: {
    ...ptLocale,
  },
};

export const getLocale = () => {
  const cookieLanguage = getLanguage();
  if (cookieLanguage) {
    return cookieLanguage;
  }
  const language = navigator.language.toLowerCase();
  const locales = Object.keys(messages);
  for (const locale of locales) {
    if (language.indexOf(locale) > -1) {
      return locale;
    }
  }

  // Default language is English
  return "en-gb";
};

const i18n = createI18n({
  locale: getLocale(),
  fallbackLocale: "en-gb",
  messages: messages,
});

export default i18n;
