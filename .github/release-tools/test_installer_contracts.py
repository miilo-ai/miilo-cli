from pathlib import Path
import unittest
import xml.etree.ElementTree as ET

class InstallerContract(unittest.TestCase):
    def test_msi_installs_both_entrypoints_and_license(self):
        root=Path(__file__).resolve().parents[2]
        tree=ET.parse(root/'crates/miilo-cli/wix/main.wxs')
        ns={'w':'http://schemas.microsoft.com/wix/2006/wi'}
        names={node.attrib.get('Name') for node in tree.findall('.//w:File',ns)}
        self.assertTrue({'miilo.exe','miilo-local.exe','LICENSE'}<=names)
        refs={node.attrib['Id'] for node in tree.findall('.//w:ComponentRef',ns)}
        self.assertTrue({'binary0','binary1','License'}<=refs)

if __name__=='__main__':unittest.main()
