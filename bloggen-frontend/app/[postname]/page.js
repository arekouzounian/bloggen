import fs from 'fs';
import path from 'path';

import Footer from '../components/Footer';
import MetaHolder from '../components/MetaHolder';

export const dynamic = 'force-static';
export const dynamicParams = true;
export const revalidate = 60;

export async function generateStaticParams() {
  let p = path.resolve(process.cwd(), 'app', 'static');
  let dirs = fs.readdirSync(p).filter((dir) => {
    if (!fs.statSync(path.join(p, dir)).isDirectory()) { return false; }

    let inner_p = path.join(p, dir);
    let inner_files = fs.readdirSync(inner_p);
    for (var file of inner_files) {
      if (file.endsWith('.html')) {
        return true;
      }
    }
    return false;
  })

  let x = dirs.map((dir) => ({
    postname: dir
  }));
  return x;
}

export default function Page({ params }) {
  let { postname } = params;
  postname = decodeURIComponent(postname);

  let post_dir = path.resolve(process.cwd(), 'app', 'static', postname);
  let exists = fs.existsSync(post_dir);

  if (!exists) {
    return <h1>Page not found</h1>;
  }

  let files = fs.readdirSync(post_dir);
  let post_file = files.find((file) => {
    return file.endsWith('.html');
  })
  let data = fs.readFileSync(path.join(post_dir, post_file)).toString();


  let metaPath = path.join(post_dir, "meta.json");
  exists = fs.existsSync(metaPath);
  let js = null;
  if (exists) {
    let meta = fs.readFileSync(path.join(post_dir, "meta.json")).toString();
    js = JSON.parse(meta);
  }

  let title = postname;
  if (js != null) {
    title = js.Title;
  }


  const pubPath = path.resolve(process.cwd(), 'public', 'assets');
  let assPath = path.join(post_dir, "assets");
  let assets = fs.readdirSync(assPath);
  assets.forEach((asset) => {
    let final_path = path.join(pubPath, asset);
    if (!fs.existsSync(final_path)) {
      fs.copyFileSync(path.join(assPath, asset), path.join(pubPath, asset));
    }
  });

  return (
    <div className="dark:bg-blue-850 dark:text-white border-2 rounded-md p-4 m-2 shadow-lg">
      <h1 className="text-3xl font-bold underline text-center m-3 ">{title}</h1>
      <MetaHolder data={js} />
      <div className='content-container' dangerouslySetInnerHTML={{ __html: data }}></div>
      <a href="/" className="flex flex-col items-center font-medium text-slate-500 dark:text-gray hover:underline">back</a>
      <Footer />
    </div>
  );

}
