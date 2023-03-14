const { description } = require("../../package");

module.exports = {
  title: "Shura",
  description: description,
  theme: 'thindark',
  head: [
    ["meta", { name: "theme-color", content: "#3eaf7c" }],
    ["meta", { name: "apple-mobile-web-app-capable", content: "yes" }],
    [
      "meta",
      { name: "apple-mobile-web-app-status-bar-style", content: "black" },
    ],
  ],

  themeConfig: {
    repo: "",
    editLinks: false,
    docsDir: "",
    editLinkText: "",
    lastUpdated: false,
    author: {
      name: "Benjamin Hansen",
      twitter: "https://twitter.com/sotrh760",
    },
    nav: [
      {
        text: "About",
        link: "/",
      },
      {
        text: "Documentation",
        link: "/docs/",
      },
      {
        text: "Repository",
        link: "https://github.com/AndriBaal/shura",
      },
      {
        text: "Demo",
        link: "/demo/",
      },
    ],
    sidebar: {
      '/docs/': [
        {
          title: 'Documentation',
          collapsable: false,
          children: [
            '',
            'using-vue',
          ]
        }
      ],
    }
  },

  plugins: ["@vuepress/plugin-back-to-top", "@vuepress/plugin-medium-zoom"],
};
