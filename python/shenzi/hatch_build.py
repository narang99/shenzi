# reference: https://discuss.python.org/t/how-to-make-a-pure-python-wheel-for-2-7-3-6/71858/2
import sysconfig

from hatchling.builders.hooks.plugin.interface import BuildHookInterface


class CustomHook(BuildHookInterface):
    def initialize(self, version, build_data):
        if self.target_name not in ('wheel', 'bdist'):
            return

        # https://peps.python.org/pep-0425/#platform-tag
        platform_tag = sysconfig.get_platform().replace('-','_').replace('.','_')

        build_data['tag'] = f'py3-none-{platform_tag}'
