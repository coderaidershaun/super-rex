// Proves the guard's other rules: writes over a disk, recursive owner and mode changes of the home folder, stops of the machine, cloud deletes and forced pushes to main are stopped, and the near misses of each run.
import { expect, test } from 'claude-code/testing'

import { guardAlert, mayAlarm } from './index'
import type { RmPlaces } from './index'

const PLACES: RmPlaces = {
  cwd: '/Users/sam/Code/app',
  home: '/Users/sam',
  repo: '/Users/sam/Code/app',
  parent: '/Users/sam/Code',
  temp: ['/tmp', '/private/tmp', '/var/folders/x1/T'],
}

function stopped(command: string): string | null {
  const alert = guardAlert(command, PLACES)
  return alert === null ? null : `${alert.what}: ${alert.reason}`
}

function passing(commands: readonly string[]): readonly string[] {
  return commands.filter(command => stopped(command) !== null)
}

function missed(commands: readonly string[]): readonly string[] {
  return commands.filter(command => stopped(command) === null)
}

test('a write over a disk, a new file system or partition table on a device, and a disk erase are stopped', () => {
  expect(stopped('dd if=/dev/zero of=/dev/sda bs=1M')).toBe('a dd to /dev/sda: writes over a disk device, and what was on it is lost')
  expect(stopped('sudo dd if=ubuntu.iso of=/dev/rdisk2 bs=4m')).toBe('a dd to /dev/rdisk2: writes over a disk device, and what was on it is lost')
  expect(stopped('cat image.bin > /dev/disk2')).toBe('a redirect to /dev/disk2: writes over a disk device, and what was on it is lost')
  expect(stopped('mkfs.ext4 /dev/nvme0n1p1')).toBe('a mkfs.ext4 of /dev/nvme0n1p1: erases a file system, and what was on it is lost')
  expect(stopped('diskutil eraseDisk APFS Blank disk2')).toBe('a diskutil eraseDisk: erases a disk or a volume')
  expect(stopped('diskutil apfs deleteContainer disk3')).toBe('a diskutil apfs deleteContainer: erases a disk or a volume')
  const caught = ['echo x > /dev/sda', 'printf x >> /dev/nvme0n1', 'echo x | sudo tee /dev/sdb', 'mkfs -t ext4 /dev/sdb1', 'newfs_apfs /dev/disk4s1', 'wipefs -a /dev/sdc', 'wipefs -a "$DEV"', 'sudo fdisk /dev/sda', 'sfdisk /dev/sda < layout.txt', 'parted -s /dev/sda mklabel gpt', 'sgdisk --zap-all /dev/sda', 'gpt destroy disk2', 'shred -n 3 /dev/sda', 'diskutil zeroDisk disk2', 'diskutil secureErase 0 disk2', 'diskutil partitionDisk disk2 GPT APFS x 100%']

  expect(missed(caught)).toEqual([])
})

test('the near misses of the disk rules run: other device files, image files, and the commands that only read a disk', () => {
  const passed = ['dd if=/dev/zero of=disk.img bs=1M count=100', 'dd if=/dev/sda of=backup.img', 'dd if=x of=/dev/null', 'cmd > /dev/null 2>&1', 'echo hi > /dev/stderr', 'echo hi | tee /dev/tty', 'mkfs.ext4 -F disk.img', 'fdisk -l', 'fdisk -l /dev/sda', 'sfdisk -d /dev/sda > layout.txt', 'parted /dev/sda print', 'parted -l', 'diskutil list', 'diskutil info disk2', 'diskutil apfs list', 'ls -l /dev/disk2', 'cat /dev/urandom | head -c 16', 'shred -u secrets.txt']

  expect(passing(passed)).toEqual([])
})

test('a recursive chmod, chown or chgrp of the root, the home folder or a folder that holds it is stopped, and one of a folder below the home folder runs', () => {
  expect(stopped('chmod -R 777 /')).toBe('a recursive chmod of /: changes every file below the root folder')
  expect(stopped('sudo chown -R sam:staff ~')).toBe('a recursive chown of /Users/sam: changes every file below your home folder')
  expect(stopped('chgrp --recursive staff /Users')).toBe('a recursive chgrp of /Users: changes every file below a folder that holds your home folder')
  expect(missed(['chmod -R -w $HOME', 'chmod -Rv u+w ~/', 'cd ~ && chown -R sam .', 'chmod --recursive go-rwx /Users/sam'])).toEqual([])

  expect(passing(['chmod -R u+w target', 'chown -R sam ~/Code/app', 'chmod 700 ~', 'chmod +x ~/bin/tool', 'chown sam ~', 'chmod -R 755 ./scripts'])).toEqual([])
})

test('a stop of the machine, a kill of every process, a crontab -r and a fork bomb are stopped', () => {
  expect(stopped('sudo shutdown -h now')).toBe('a shutdown: stops the machine')
  expect(stopped('reboot')).toBe('a reboot: restarts the machine')
  expect(stopped('kill -9 -1')).toBe('a kill -9 -1: stops every process of your user')
  expect(stopped('crontab -r')).toBe('a crontab -r: deletes every cron job of your user')
  expect(stopped(':(){ :|:& };:')).toBe('a fork bomb: starts processes until the machine stops answering')
  expect(missed(['halt', 'poweroff', 'init 0', 'sudo init 6', 'systemctl reboot', 'kill -KILL -1', 'kill -s KILL -1', 'kill -- -1', 'bomb(){ bomb | bomb & }; bomb', 'crontab -ri'])).toEqual([])

  expect(passing(['kill -9 1234', 'kill -1234', 'kill %1', 'pkill -f vite', 'crontab -l', 'git init', 'npm init -y', 'systemctl restart nginx', 'f() { echo hi; }; f', 'say "shutdown later"'])).toEqual([])
})

test('the cloud and infrastructure deletes of the table are stopped, through npx, bundle exec and their kind', () => {
  const caught = [
    'terraform destroy -auto-approve',
    'terraform -chdir=infra apply -destroy',
    'tofu destroy',
    'pulumi destroy --yes',
    'kubectl delete namespace prod',
    'kubectl delete ns staging',
    'kubectl delete pods --all -n prod',
    'kubectl delete deploy --all-namespaces --all',
    'aws s3 rb s3://bucket --force',
    'aws s3 rm s3://bucket/ --recursive',
    'aws --profile prod rds delete-db-instance --db-instance-identifier main',
    'aws rds delete-db-cluster --db-cluster-identifier c',
    'aws ec2 terminate-instances --instance-ids i-1',
    'aws dynamodb delete-table --table-name users',
    'gcloud projects delete my-proj',
    'gcloud sql instances delete main',
    'gcloud compute instances delete vm-1 --zone x',
    'az group delete -n rg-prod --yes',
    'docker system prune -af --volumes',
    'docker volume rm pgdata',
    'docker volume prune -f',
    'heroku apps:destroy -a shop --confirm shop',
    'fly apps destroy shop',
    'flyctl apps destroy shop -y',
    'gh repo delete owner/repo --yes',
    'supabase db reset',
    'npx prisma migrate reset --force',
    'pnpm exec prisma migrate reset',
    'yarn prisma migrate reset',
    'bin/rails db:drop',
    'bundle exec rake db:drop',
    'dropuser app',
  ]

  expect(missed(caught)).toEqual([])
  expect(stopped('terraform destroy')).toBe('a terraform destroy: deletes the infrastructure it manages')
  expect(stopped('kubectl delete pods --all -n prod')).toBe('a kubectl delete --all: deletes every resource of a kind')
})

test('the near misses of the cloud table run: plans, lists, one named resource, and commands the table leaves out', () => {
  const passed = [
    'terraform plan -destroy',
    'terraform apply',
    'kubectl delete pod web-1 -n ns',
    'kubectl delete -f ns.yaml',
    'kubectl get ns',
    'aws s3 rm s3://bucket/key.txt',
    'aws s3 ls',
    'aws s3 cp x s3://b/rb',
    'gcloud projects list',
    'docker system prune -f',
    'docker volume ls',
    'docker rm -f web',
    'helm uninstall web',
    'gh repo view',
    'supabase db push',
    'npx prisma migrate dev',
    'npm run db:reset',
    'rails db:migrate',
    'fly deploy',
  ]

  expect(passing(passed)).toEqual([])
})

test('a forced push or a remote delete of main or master is stopped; one of another branch, or with no branch named, runs', () => {
  expect(stopped('git push --force origin main')).toBe('a forced push to main: rewrites the history of the branch that every agent and you share, and commits on it may be lost')
  expect(stopped('git push origin :master')).toBe('a delete of the remote branch master: deletes the branch that every agent and you share')
  const caught = ['git push -f origin master', 'git push origin main --force-with-lease', 'git push origin +main', 'git push origin feat:main -f', 'git push origin +HEAD:refs/heads/master', 'git push --delete origin main', 'git push origin -d master', 'git -C ../app push -f origin main']

  expect(missed(caught)).toEqual([])
  expect(passing(['git push origin main', 'git push -f origin feat/x', 'git push --force', 'git push -u origin HEAD', 'git push origin --delete feat/x', 'git push origin :feat/x', 'git push origin +feat/x'])).toEqual([])
})

test('a command with no trigger word costs one test and no more', () => {
  expect(['ls -la', 'cargo test --workspace', 'git status', 'npm run build', 'cat README.md', 'git push origin rex/0gaq820001', 'git push -f -u origin feat/x'].filter(mayAlarm)).toEqual([])
})
