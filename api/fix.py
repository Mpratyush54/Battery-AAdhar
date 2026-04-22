import os, glob

dirs = ['.', 'routes', 'grpc', 'config', 'controllers', 'middleware', 'services', 'models', 'tests']
for d in dirs:
    for f in glob.glob(d + '/*.go'):
        try:
            with open(f, 'r', encoding='utf-8') as ifile:
                content = ifile.read()
            content = content.replace('"api/', '"github.com/Mpratyush54/Battery-AAdhar/api/')
            with open(f, 'w', encoding='utf-8') as ofile:
                ofile.write(content)
        except Exception as e:
            pass
