import fs from 'fs';
import path from 'path';

import Footer from '../../components/Footer';

export const dynamic = 'auto';
export const dynamicParams = true;
export const revalidate = 10; 

export async function generateStaticParams() {
  let p = path.resolve(process.cwd(), 'app', 'static');
  let dirs = fs.readdirSync(p).filter((dir) => {
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
  
  return (
    <div className="dark:bg-blue-850 dark:text-white">
      <h1 className="text-3xl font-bold underline text-center m-3 ">{postname}</h1>
      <div className='content-container border-2 rounded-md p-4 m-2' dangerouslySetInnerHTML={{__html: data}}></div>
      <a href="/posts" className="flex flex-col items-center font-medium text-slate-500 dark:text-gray hover:underline">back</a>
      <Footer />
    </div>
  );

}