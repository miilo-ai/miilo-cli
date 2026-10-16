"""Missing configuration skips publication; failed validation is never swallowed."""
import os


def permitted(env):
    requested = env.get('MIILO_RELEASE_ENABLED') == 'true' or env.get('MIILO_PUBLISH') == 'true'
    return (requested and env.get('GITHUB_EVENT_NAME') != 'pull_request'
            and bool(env.get('RELEASE_TOKEN', '').strip())
            and env.get('DEPENDENCIES_READY', 'true') == 'true')


def main():
    enabled = permitted(os.environ)
    value = str(enabled).lower()
    print('Publication authorized for this workflow' if enabled else 'No release has been made')
    if output := os.environ.get('GITHUB_OUTPUT'):
        with open(output, 'a') as stream:
            stream.write(f'publish={value}\n')
    if not enabled and (summary := os.environ.get('GITHUB_STEP_SUMMARY')):
        with open(summary, 'a') as stream:
            stream.write('No release has been made\n')


if __name__ == '__main__':
    main()
