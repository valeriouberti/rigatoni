# Rigatoni Documentation

This directory contains the Rigatoni project documentation, published to GitHub Pages using Jekyll.

> **📖 About This File**
>
> This README is a **meta-document for contributors** explaining how to work with the documentation system (Jekyll, GitHub Pages, etc.).
>
> **Users** should visit the published documentation at [valeriouberti.github.io/rigatoni](https://valeriouberti.github.io/rigatoni) or view [index.md](index.md) for the documentation landing page.

## Documentation Structure

```
docs/
├── _config.yml              # Jekyll configuration
├── index.md                 # Landing page
├── getting-started.md       # Quick start guide
├── architecture.md          # Architecture documentation
├── contributing.md          # Contribution guidelines
├── guides/                  # User guides
│   ├── index.md            # Guides landing page
│   ├── s3-configuration.md # S3 configuration guide
│   └── production-deployment.md  # Production deployment guide
└── api/                     # API reference
    └── index.md            # API reference landing page
```

## Viewing Documentation Locally

### Option 1: Using Jekyll (Recommended)

Install Jekyll:

```bash
# macOS
brew install ruby
gem install bundler jekyll

# Ubuntu/Debian
sudo apt-get install ruby-full build-essential
gem install bundler jekyll
```

Serve the docs:

```bash
cd docs
jekyll serve --baseurl ''

# Open browser to http://localhost:4000
```

### Option 2: Using Python HTTP Server

```bash
cd docs
python3 -m http.server 8000

# Open browser to http://localhost:8000
```

Note: This won't process Jekyll templates, so some features may not work.

## Publishing to GitHub Pages

### Enable GitHub Pages

1. Go to repository settings
2. Navigate to "Pages" section
3. Set source to "Deploy from a branch"
4. Select branch: `main`
5. Select folder: `/docs`
6. Save

Documentation will be available at: `https://valeriouberti.github.io/rigatoni`

### Auto-Deployment

GitHub Pages automatically rebuilds documentation when changes are pushed to the `main` branch in the `docs/` directory.

## Documentation Guidelines

### Writing Style

- Use clear, concise language
- Include code examples for all features
- Add tables of contents for long pages
- Use callouts for important information
- Keep pages focused on a single topic

### Markdown Format

Follow these conventions:

```markdown
---
layout: default
title: Page Title
nav_order: 1
description: "Page description for SEO"
permalink: /page-url
---

# Page Title
{: .no_toc }

Brief description.
{: .fs-6 .fw-300 }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Section

Content here...
```

### Code Blocks

Use language-specific syntax highlighting:

````markdown
```rust
fn main() {
    println!("Hello, world!");
}
```

```bash
cargo build --release
```

```toml
[dependencies]
rigatoni = "0.1"
```
````

### Callouts

Use Jekyll callouts for important information:

```markdown
{: .note }
This is a note.

{: .warning }
This is a warning.

{: .important }
This is important information.
```

### Links

Use relative links for internal documentation:

```markdown
[Getting Started](getting-started)
[Architecture Guide](architecture)
[S3 Configuration](guides/s3-configuration)
```

Use absolute links for external resources:

```markdown
[Rust Documentation](https://doc.rust-lang.org/)
[MongoDB Change Streams](https://www.mongodb.com/docs/manual/changeStreams/)
```

## Adding New Documentation

### New Guide

1. Create a new file in `guides/`:

```bash
touch docs/guides/new-guide.md
```

2. Add frontmatter:

```markdown
---
layout: default
title: Guide Title
parent: Guides
nav_order: 3
description: "Guide description"
---
```

3. Update `guides/index.md` to link to the new guide

### New Top-Level Page

1. Create file in `docs/`:

```bash
touch docs/new-page.md
```

2. Add frontmatter with `nav_order`:

```markdown
---
layout: default
title: New Page
nav_order: 7
permalink: /new-page
---
```

## Theme Customization

The documentation uses the `just-the-docs` theme. Configuration is in `_config.yml`.

### Color Schemes

Available schemes:
- `light` (default)
- `dark`
- `custom`

Change in `_config.yml`:

```yaml
color_scheme: dark
```

### Navigation

Control navigation order with `nav_order`:

```yaml
nav_order: 1  # Appears first
nav_order: 2  # Appears second
# etc.
```

### Search

Search is enabled by default. Configure in `_config.yml`:

```yaml
search_enabled: true
search:
  heading_level: 2
  previews: 3
```

## Updating Documentation

### Regular Updates

1. Make changes to markdown files
2. Test locally with Jekyll
3. Commit and push to `main`
4. GitHub Pages rebuilds automatically

### Version-Specific Documentation

For major versions, consider:

1. Creating version branches (e.g., `v0.1-docs`, `v0.2-docs`)
2. Using subdirectories (e.g., `docs/v0.1/`, `docs/v0.2/`)
3. Adding version selector in navigation

## Troubleshooting

### Jekyll Build Fails

Check Jekyll version compatibility:

```bash
bundle exec jekyll --version
```

### Pages Not Updating

1. Check GitHub Actions for build errors
2. Verify `_config.yml` is valid YAML
3. Check that frontmatter is properly formatted
4. Clear browser cache

### Broken Links

Use a link checker:

```bash
# Install linkchecker
pip install linkchecker

# Check links
linkchecker http://localhost:4000
```

### Search Not Working

1. Verify `search_enabled: true` in `_config.yml`
2. Rebuild site: `jekyll clean && jekyll serve`
3. Check browser console for errors

## Resources

- **Jekyll Documentation**: https://jekyllrb.com/docs/
- **Just the Docs Theme**: https://just-the-docs.github.io/just-the-docs/
- **GitHub Pages**: https://docs.github.com/en/pages
- **Markdown Guide**: https://www.markdownguide.org/

## Questions?

For documentation questions, open an issue or discussion on GitHub.
