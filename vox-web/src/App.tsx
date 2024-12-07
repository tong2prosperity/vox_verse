import React from 'react';
import { ThemeProvider, createTheme } from '@mui/material/styles';
import CssBaseline from '@mui/material/CssBaseline';
import { VideoChat } from './components/VideoChat';
import { User } from './types/types';

const darkTheme = createTheme({
  palette: {
    mode: 'dark',
  },
});

// 模拟用户数据
const currentUser: User = {
  id: 'user-' + Math.random().toString(36).substr(2, 9),
  name: 'User ' + Math.floor(Math.random() * 1000)
};

function App() {
  return (
    <ThemeProvider theme={darkTheme}>
      <CssBaseline />
      <VideoChat user={currentUser} />
    </ThemeProvider>
  );
}

export default App;
