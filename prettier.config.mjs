const config = {
  semi: false,
  trailingComma: 'all',
  singleQuote: true,
  plugins: [],
  overrides: [
    {
      files: '*.yaml',
      options: { singleQuote: true },
    },
  ],
}

export default config
