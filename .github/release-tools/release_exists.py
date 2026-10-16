"""An existing immutable release is a no-op, not permission to overwrite its assets."""
import json
import os
import urllib.error
import urllib.parse
import urllib.request


def main():
    tag=os.environ.get('RELEASE_TAG','')
    token=os.environ.get('RELEASE_TOKEN','')
    exists=False
    if token and tag:
        url='https://api.github.com/repos/'+os.environ['GITHUB_REPOSITORY']+'/releases/tags/'+urllib.parse.quote(tag,safe='')
        request=urllib.request.Request(url,headers={'Authorization':'Bearer '+token,'User-Agent':'miilo-release-check','Accept':'application/vnd.github+json'})
        try:
            with urllib.request.urlopen(request,timeout=20) as response:json.load(response)
            exists=True
        except urllib.error.HTTPError as error:
            if error.code!=404:raise RuntimeError('Release lookup failed; publication is not authorized') from None
    with open(os.environ['GITHUB_OUTPUT'],'a') as stream:stream.write(f'exists={str(exists).lower()}\n')


if __name__=='__main__':main()
