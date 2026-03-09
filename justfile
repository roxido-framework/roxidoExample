install target = ".":
  R CMD INSTALL {{target}}

build:
  roxido build

expand:
  Rscript -e "cmd <- 'roxido expand --color always'; if (nzchar(Sys.which('bat'))) cmd <- paste(cmd, '| bat'); system(cmd)"

roxygen2:
  Rscript -e "roxygen2::roxygenise()"

api:
  roxido api

delete-release tag:
  git push --delete origin {{tag}}
  git tag -d {{tag}}

date := datetime("%y.%m.%d")

new-release: check-clean
  -just delete-release v{{date}}
  -just delete-release latest
  sed -i 's|^Config/Roxido/TemplateVersion: .*$|Config/Roxido/TemplateVersion: {{date}}|' DESCRIPTION   # Delete
  git add DESCRIPTION                                                                                   # Delete
  git commit -m "New release: v{{date}}" || true                                                        # Delete
  git tag v{{date}}
  git tag latest
  git push
  git push --tags

check-clean:
  git diff --quiet --exit-code
  git diff --quiet --exit-code --cached
