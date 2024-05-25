import fs from 'fs';
import path from 'path';

export const dynamicParams = false;
export const revalidate = 10; 

export async function generateStaticParams() {
  let p = path.resolve('.', 'app', 'static');
  let dirs = fs.readdirSync(p).filter((dir) => {
    let inner_p = path.resolve('.', 'app', 'static', dir); 
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
  console.log(params);
  const { postname } = params; 


  let post_dir = path.resolve('.', 'app', 'static', postname);
  let files = fs.readdirSync(post_dir);
  let post_file = files.find((file) => {
    return file.endsWith('.html');
  })

  let data = fs.readFileSync(path.resolve('.', 'app', 'static', postname, post_file)).toString();
  
  return (
    <div>
    <h1 className="text-3xl font-bold underline">you made it to post '{postname}'</h1>
    <div className='content-container border-2 p-2' dangerouslySetInnerHTML={{__html: data}}></div>
    </div>
  );

}