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

const Plans = [
  {
    id: 1,
    icon: "",
    for: "individuals",
    type: "free",
    price: "Free",
    duration: "Month",
    included: {
      rule1: "200GB Ingestion",
      rule2: "200GB Query Volume",
      rule3: "15 Days Retention",
      rule4: "10 users",
    },
    subscribed: "Subscribed",
    getStarted: "Get started",
    color: "secondary",
  },
  {
    id: 2,
    icon: "",
    for: "startups",
    type: "pay-as-you-go",
    price: "$19",
    duration: "Month",
    included: {
      rule1: "Additional Ingestion ($0.3/GB)",
      rule2: "Additional Query Volume ($0.01/GB)",
      rule3: "30 days retention for logs and traces",
      rule4: "120 days retention for metrics",
      rule5: "Unlimited users",
    },
    subscribed: "Subscribed",
    getStarted: "Get started",
    color: "primary",
  },
  {
    id: 3,
    icon: "",
    for: "business",
    type: "enterprise",
    price: "$199",
    duration: "Month",
    included: {
      rule1: "Additional Ingestion ($0.3/GB)",
      rule2: "Additional Query Volume ($0.01/GB)",
      rule3: "60 days retention for logs and traces",
      rule4: "13 months retention for metrics",
      rule5: "Unlimited users",
    },
    subscribed: "Subscribed",
    getStarted: "Get started",
    color: "primary",
  },
];

export default Plans;
