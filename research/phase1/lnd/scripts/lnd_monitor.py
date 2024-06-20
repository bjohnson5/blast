# apt install -y libgtk-3-common libasound2 libdbus-glib-1-2
# mkdir -p /dir && cd /dir && wget -O - https://ftp.mozilla.org/pub/firefox/releases/105.0/linux-x86_64/en-US/firefox-105.0.tar.bz2 | tar -xjf -
# PATH=/dir/firefox/:$PATH
# pip install bs4
# pip install selenium

import subprocess
import sys
import time
import csv
from bs4 import BeautifulSoup
from selenium import webdriver  
from selenium.webdriver import FirefoxOptions

def get_html(url):
        opts = FirefoxOptions()
        opts.add_argument("--headless")
        browser = webdriver.Firefox(options=opts)
        browser.get(url)
        html = browser.page_source
        browser.quit()
        return html

pprof_headers = ['Time', 'Flat', 'Flat%', 'Sum%', 'Cum', 'Cum%', 'Name', 'Inlined?']
top_headers = ["Time", "USER", "PID", "%CPU", "%MEM", "VSZ", "RSS", "TTY", "STAT", "START", "TIME"]
pprof_url = "http://localhost:6060/debug/pprof/allocs"
pprof_table = "toptable"
pprof_data = []
top_data = []

for i in range(int(sys.argv[1])):
        # PPROF
        proc = subprocess.Popen(["go","tool","pprof", "--http=:8082", pprof_url])
        html = get_html('http://localhost:8082/ui/top')
        proc.terminate()

        soup = BeautifulSoup(html, 'html.parser')
        table = soup.find(attrs={'id':pprof_table})

        tbody = table.find_all('tbody')[0]
        rows = tbody.find_all('tr')
        for row in rows:
            cols = row.find_all('td')
            cols = [ele.text.strip() for ele in cols]
            pprof_data.append([ele for ele in cols if ele])
            pprof_data[len(pprof_data)-1].insert(0, i)

        # TOP
        ps_process = subprocess.Popen(['ps', 'aux'], stdout=subprocess.PIPE)
        grep_process = subprocess.Popen(['grep', 'lnd'],stdin=ps_process.stdout, stdout=subprocess.PIPE)
        ps_process.stdout.close()
        out, err = grep_process.communicate()

        output = out.decode('utf-8').split("lnd --profile=")[0]
        top_data.append([x.strip() for x in output.split(" ") if x != ''])
        top_data[len(top_data)-1].insert(0, i)

        time.sleep(1)

with open('pprof.csv', 'w') as file:
        writer = csv.writer(file)
        writer.writerow(pprof_headers)
        writer.writerows(pprof_data)

with open('top.csv', 'w') as file:
        writer = csv.writer(file)
        writer.writerow(top_headers)
        writer.writerows(top_data)
